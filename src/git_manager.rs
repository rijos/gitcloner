use anyhow::{Result, anyhow};
use git2::{Repository, FetchOptions, RemoteCallbacks};
use std::path::PathBuf;
use std::fs;
use tokio::task;
use tracing::{info, warn};
use crate::models::Repository as RepoModel;

#[derive(Clone)]
pub struct GitManager {
    base_path: PathBuf,
}

impl GitManager {
    pub async fn new(base_path: String) -> Result<Self> {
        let path = PathBuf::from(base_path);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(GitManager { base_path: path })
    }

    pub async fn clone_repository(&self, url: &str) -> Result<String> {
        let url = url.to_string();
        let base_path = self.base_path.clone();
        
        task::spawn_blocking(move || {
            let repo_name = extract_repo_name(&url)?;
            let local_path = base_path.join(&repo_name);
            
            if local_path.exists() {
                return Err(anyhow!("Repository already exists at {}", local_path.display()));
            }

            info!("Cloning repository {} to {}", url, local_path.display());
            
            let mut callbacks = RemoteCallbacks::new();
            callbacks.update_tips(|refname, a, b| {
                if a.is_zero() {
                    info!("Creating reference {}", refname);
                } else {
                    info!("Updating reference {} from {} to {}", refname, a, b);
                }
                true
            });
            
            callbacks.pack_progress(|_stage: git2::PackBuilderStage, _transferred: usize, _total: usize| {
                // Progress tracking callback
            });

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_options);
            
            builder.clone(&url, &local_path)?;
            
            Ok(local_path.to_string_lossy().to_string())
        }).await?
    }

    pub async fn sync_repository(&self, repo: &RepoModel) -> Result<()> {
        let url = repo.url.clone();
        let local_path = PathBuf::from(&repo.local_path);
        
        task::spawn_blocking(move || {
            if !local_path.exists() {
                return Err(anyhow!("Repository path does not exist: {}", local_path.display()));
            }

            info!("Syncing repository {} at {}", url, local_path.display());
            
            let repo = Repository::open(&local_path)?;
            
            // Get the remote (usually 'origin')
            let mut remote = repo.find_remote("origin")?;
            
            // Create callbacks for progress tracking
            let mut callbacks = RemoteCallbacks::new();
            callbacks.update_tips(|refname, a, b| {
                if a.is_zero() {
                    info!("Creating reference {}", refname);
                } else {
                    info!("Updating reference {} from {} to {}", refname, a, b);
                }
                true
            });
            
            // Fetch from remote without merging/overriding local changes
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);
            
            remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], Some(&mut fetch_options), None)?;
            
            // Check if there are local changes
            let statuses = repo.statuses(None)?;
            if !statuses.is_empty() {
                warn!("Repository {} has local changes, skipping merge to preserve local history", url);
                return Ok(());
            }
            
            // Get the current branch
            let head = repo.head()?;
            if let Some(branch_name) = head.shorthand() {
                // Try to fast-forward merge if possible
                let remote_branch_name = format!("origin/{}", branch_name);
                if let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/{}", remote_branch_name)) {
                    let remote_commit = remote_ref.peel_to_commit()?;
                    let local_commit = head.peel_to_commit()?;
                    
                    // Check if we can fast-forward
                    let (ahead, behind) = repo.graph_ahead_behind(local_commit.id(), remote_commit.id())?;
                    
                    if ahead == 0 && behind > 0 {
                        // We can fast-forward
                        info!("Fast-forwarding {} commits in {}", behind, url);
                        let mut reference = repo.find_reference(&format!("refs/heads/{}", branch_name))?;
                        reference.set_target(remote_commit.id(), "Fast-forward merge")?;
                        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                    } else if ahead > 0 && behind > 0 {
                        warn!("Repository {} has diverged from remote, skipping merge to preserve local history", url);
                    } else {
                        info!("Repository {} is up to date", url);
                    }
                }
            }
            
            Ok(())
        }).await?
    }
}

fn extract_repo_name(url: &str) -> Result<String> {
    let url = url.trim_end_matches('/');
    
    // Parse the URL to extract host, org, and repo name
    let parsed_url = if url.starts_with("http://") || url.starts_with("https://") {
        // HTTP/HTTPS URL
        let without_protocol = url.split("://").nth(1)
            .ok_or_else(|| anyhow!("Invalid URL format"))?;
        
        let parts: Vec<&str> = without_protocol.split('/').collect();
        if parts.len() < 3 {
            return Err(anyhow!("Invalid repository URL format"));
        }
        
        let host = parts[0];
        let org = parts[1];
        let repo = parts[2].trim_end_matches(".git");
        
        // Create fully qualified name: host/org/repo
        format!("{}/{}/{}", host, org, repo)
    } else if url.contains("@") {
        // SSH URL like git@github.com:user/repo.git
        let parts: Vec<&str> = url.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid SSH URL format"));
        }
        
        let host_and_path = parts[1];
        let colon_split: Vec<&str> = host_and_path.split(':').collect();
        if colon_split.len() != 2 {
            return Err(anyhow!("Invalid SSH URL format"));
        }
        
        let host = colon_split[0];
        let path = colon_split[1];
        let path_parts: Vec<&str> = path.split('/').collect();
        
        if path_parts.len() == 2 {
            // org/repo format
            let org = path_parts[0];
            let repo = path_parts[1].trim_end_matches(".git");
            format!("{}/{}/{}", host, org, repo)
        } else if path_parts.len() == 1 {
            // just repo format
            let repo = path_parts[0].trim_end_matches(".git");
            format!("{}/{}", host, repo)
        } else {
            return Err(anyhow!("Invalid repository path format"));
        }
    } else {
        return Err(anyhow!("Unsupported URL format"));
    };
    
    if parsed_url.is_empty() {
        return Err(anyhow!("Could not extract repository name from URL"));
    }
    
    // Replace any invalid filesystem characters
    let safe_name = parsed_url.replace(":", "_").replace("@", "_");
    
    Ok(safe_name)
}
