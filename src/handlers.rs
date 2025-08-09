use warp::{Filter, Reply, Rejection};
use serde_json::json;
use crate::auth::AuthManager;
use crate::database::Database;
use crate::git_manager::GitManager;
use crate::models::{LoginRequest, AddRepositoryRequest, ApiResponse};

lazy_static::lazy_static! {
    static ref AUTH_MANAGER: AuthManager = AuthManager::new();
}

pub fn auth_routes(db: Database) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    login(db.clone())
        .or(logout())
}

pub fn repo_routes(db: Database, git_manager: GitManager) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_repositories(db.clone())
        .or(add_repository(db.clone(), git_manager.clone()))
        .or(remove_repository(db.clone()))
        .or(sync_repository(db, git_manager))
}

fn login(db: Database) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("auth" / "login")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db))
        .and_then(handle_login)
}

fn logout() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("auth" / "logout")
        .and(warp::post())
        .and(with_auth_token())
        .and_then(handle_logout)
}

fn get_repositories(db: Database) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("repositories")
        .and(warp::get())
        .and(with_auth())
        .and(with_db(db))
        .and_then(handle_get_repositories)
}

fn add_repository(db: Database, git_manager: GitManager) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("repositories")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_auth())
        .and(with_db(db))
        .and(with_git_manager(git_manager))
        .and_then(handle_add_repository)
}

fn remove_repository(db: Database) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("repositories" / String)
        .and(warp::delete())
        .and(with_auth())
        .and(with_db(db))
        .and_then(handle_remove_repository)
}

fn sync_repository(db: Database, git_manager: GitManager) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("repositories" / String / "sync")
        .and(warp::post())
        .and(with_auth())
        .and(with_db(db))
        .and(with_git_manager(git_manager))
        .and_then(handle_sync_repository)
}

fn with_db(db: Database) -> impl Filter<Extract = (Database,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

fn with_git_manager(git_manager: GitManager) -> impl Filter<Extract = (GitManager,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || git_manager.clone())
}

fn with_auth() -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and_then(|auth_header: Option<String>| async move {
            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if let Some(username) = AUTH_MANAGER.validate_session(token).await {
                        Ok(username)
                    } else {
                        Err(warp::reject::custom(Unauthorized))
                    }
                }
                _ => Err(warp::reject::custom(Unauthorized)),
            }
        })
}

fn with_auth_token() -> impl Filter<Extract = (String, String), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and_then(|auth_header: Option<String>| async move {
            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if let Some(username) = AUTH_MANAGER.validate_session(token).await {
                        Ok((username, token.to_string()))
                    } else {
                        Err(warp::reject::custom(Unauthorized))
                    }
                }
                _ => Err(warp::reject::custom(Unauthorized)),
            }
        })
        .untuple_one()
}

async fn handle_login(request: LoginRequest, db: Database) -> Result<Box<dyn Reply>, Rejection> {
    match db.get_user_by_username(&request.username).await {
        Ok(Some(user)) => {
            if crate::auth::AuthManager::verify_password(&request.password, &user.password_hash).unwrap_or(false) {
                let token = AUTH_MANAGER.create_session(&user.username).await;
                let response = ApiResponse {
                    success: true,
                    data: Some(json!({
                        "token": token,
                        "username": user.username
                    })),
                    message: None,
                };
                Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK)))
            } else {
                let response = ApiResponse::<()> {
                    success: false,
                    data: None,
                    message: Some("Invalid credentials".to_string()),
                };
                Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::UNAUTHORIZED)))
            }
        }
        _ => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some("Invalid credentials".to_string()),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::UNAUTHORIZED)))
        }
    }
}

async fn handle_logout(_username: String, token: String) -> Result<Box<dyn Reply>, Rejection> {
    // Remove the session from the auth manager
    AUTH_MANAGER.remove_session(&token).await;
    
    let response = ApiResponse {
        success: true,
        data: Some(json!({"message": "Logged out successfully"})),
        message: None,
    };
    Ok(Box::new(warp::reply::json(&response)))
}

async fn handle_get_repositories(_username: String, db: Database) -> Result<Box<dyn Reply>, Rejection> {
    match db.get_all_repositories().await {
        Ok(repositories) => {
            let response = ApiResponse {
                success: true,
                data: Some(repositories),
                message: None,
            };
            Ok(Box::new(warp::reply::json(&response)))
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Failed to fetch repositories: {}", e)),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::INTERNAL_SERVER_ERROR)))
        }
    }
}

async fn handle_add_repository(
    request: AddRepositoryRequest,
    _username: String,
    db: Database,
    git_manager: GitManager,
) -> Result<Box<dyn Reply>, Rejection> {
    // Extract repository name from URL
    let repo_name = match extract_repo_name(&request.url) {
        Ok(name) => name,
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Invalid repository URL: {}", e)),
            };
            return Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST)));
        }
    };
    
    match git_manager.clone_repository(&request.url).await {
        Ok(local_path) => {
            if let Err(e) = db.add_repository(&request.url, &repo_name, &local_path).await {
                let response = ApiResponse::<()> {
                    success: false,
                    data: None,
                    message: Some(format!("Failed to save repository: {}", e)),
                };
                return Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::INTERNAL_SERVER_ERROR)));
            }
            
            let response = ApiResponse {
                success: true,
                data: Some(json!({
                    "url": request.url,
                    "name": repo_name,
                    "local_path": local_path
                })),
                message: Some("Repository cloned successfully".to_string()),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::CREATED)))
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Failed to clone repository: {}", e)),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST)))
        }
    }
}

async fn handle_remove_repository(
    url: String,
    _username: String,
    db: Database,
) -> Result<Box<dyn Reply>, Rejection> {
    let url_clone = url.clone();
    let decoded_url = urlencoding::decode(&url).unwrap_or_else(|_| url_clone.into());
    
    // First, get the repository info to obtain the local path
    let repo_info = match db.get_repository_by_url(&decoded_url).await {
        Ok(Some(repo)) => repo,
        Ok(None) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some("Repository not found".to_string()),
            };
            return Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&response), 
                warp::http::StatusCode::NOT_FOUND
            )));
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Failed to get repository info: {}", e)),
            };
            return Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&response), 
                warp::http::StatusCode::INTERNAL_SERVER_ERROR
            )));
        }
    };

    // Remove the local directory if it exists
    if std::path::Path::new(&repo_info.local_path).exists() {
        if let Err(e) = std::fs::remove_dir_all(&repo_info.local_path) {
            tracing::warn!("Failed to remove directory {}: {}", repo_info.local_path, e);
            // Continue with database removal even if directory removal fails
        } else {
            tracing::info!("Removed directory: {}", repo_info.local_path);
        }
    }
    
    // Remove from database
    match db.remove_repository(&decoded_url).await {
        Ok(_) => {
            let response = ApiResponse {
                success: true,
                data: Some(json!({"message": "Repository and local files removed successfully"})),
                message: None,
            };
            Ok(Box::new(warp::reply::json(&response)))
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Failed to remove repository from database: {}", e)),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::INTERNAL_SERVER_ERROR)))
        }
    }
}

async fn handle_sync_repository(
    url: String,
    _username: String,
    db: Database,
    git_manager: GitManager,
) -> Result<Box<dyn Reply>, Rejection> {
    let url_clone = url.clone();
    let decoded_url = urlencoding::decode(&url).unwrap_or_else(|_| url_clone.into());
    
    match db.get_all_repositories().await {
        Ok(repositories) => {
            if let Some(repo) = repositories.iter().find(|r| r.url == decoded_url.as_ref()) {
                match git_manager.sync_repository(repo).await {
                    Ok(_) => {
                        let _ = db.update_repository_status(&repo.url, "synced").await;
                        let _ = db.update_last_synced(&repo.url).await;
                        
                        let response = ApiResponse {
                            success: true,
                            data: Some(json!({"message": "Repository synced successfully"})),
                            message: None,
                        };
                        Ok(Box::new(warp::reply::json(&response)))
                    }
                    Err(e) => {
                        let _ = db.update_repository_status(&repo.url, "error").await;
                        let response = ApiResponse::<()> {
                            success: false,
                            data: None,
                            message: Some(format!("Failed to sync repository: {}", e)),
                        };
                        Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::INTERNAL_SERVER_ERROR)))
                    }
                }
            } else {
                let response = ApiResponse::<()> {
                    success: false,
                    data: None,
                    message: Some("Repository not found".to_string()),
                };
                Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::NOT_FOUND)))
            }
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                message: Some(format!("Failed to fetch repositories: {}", e)),
            };
            Ok(Box::new(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::INTERNAL_SERVER_ERROR)))
        }
    }
}

fn extract_repo_name(url: &str) -> anyhow::Result<String> {
    let url = url.trim_end_matches('/');
    
    // Parse the URL to extract host, org, and repo name
    let parsed_url = if url.starts_with("http://") || url.starts_with("https://") {
        // HTTP/HTTPS URL
        let without_protocol = url.split("://").nth(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid URL format"))?;
        
        let parts: Vec<&str> = without_protocol.split('/').collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid repository URL format"));
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
            return Err(anyhow::anyhow!("Invalid SSH URL format"));
        }
        
        let host_and_path = parts[1];
        let colon_split: Vec<&str> = host_and_path.split(':').collect();
        if colon_split.len() != 2 {
            return Err(anyhow::anyhow!("Invalid SSH URL format"));
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
            return Err(anyhow::anyhow!("Invalid repository path format"));
        }
    } else {
        return Err(anyhow::anyhow!("Unsupported URL format"));
    };
    
    if parsed_url.is_empty() {
        return Err(anyhow::anyhow!("Could not extract repository name from URL"));
    }
    
    // Replace any invalid filesystem characters
    let safe_name = parsed_url.replace(":", "_").replace("@", "_");
    
    Ok(safe_name)
}

#[derive(Debug)]
struct Unauthorized;

impl warp::reject::Reject for Unauthorized {}
