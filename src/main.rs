use anyhow::Result;
use std::env;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, error};
use warp::Filter;

mod auth;
mod database;
mod git_manager;
mod handlers;
mod models;

use database::Database;
use git_manager::GitManager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:gitcloner.db".to_string());
    
    let db = Database::new(&database_url).await?;
    db.migrate().await?;

    let git_manager = GitManager::new("./repos".to_string()).await?;
    
    // Setup cron scheduler for daily sync
    let sched = JobScheduler::new().await?;
    let db_clone = db.clone();
    let git_manager_clone = git_manager.clone();
    
    sched.add(
        Job::new_async("0 0 2 * * *", move |_uuid, _l| {
            let db = db_clone.clone();
            let git_manager = git_manager_clone.clone();
            Box::pin(async move {
                info!("Starting daily repository sync");
                if let Err(e) = sync_all_repositories(&db, &git_manager).await {
                    error!("Daily sync failed: {}", e);
                }
            })
        })?
    ).await?;

    sched.start().await?;

    // Setup routes
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization"])
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTIONS"]);

    let static_files = warp::path("static")
        .and(warp::fs::dir("static"));

    let api = warp::path("api")
        .and(
            handlers::auth_routes(db.clone())
                .or(handlers::repo_routes(db.clone(), git_manager.clone()))
        );

    let index = warp::path::end()
        .and(warp::fs::file("static/index.html"));

    let routes = static_files
        .or(api)
        .or(index)
        .with(cors);

    info!("Server starting on http://localhost:3030");
    warp::serve(routes)
        .run(([0, 0, 0, 0], 3030))
        .await;

    Ok(())
}

async fn sync_all_repositories(db: &Database, git_manager: &GitManager) -> Result<()> {
    let repos = db.get_all_repositories().await?;
    for repo in repos {
        if let Err(e) = git_manager.sync_repository(&repo).await {
            error!("Failed to sync repository {}: {}", repo.url, e);
            db.update_repository_status(&repo.url, "error").await?;
        } else {
            db.update_repository_status(&repo.url, "synced").await?;
            db.update_last_synced(&repo.url).await?;
        }
    }
    Ok(())
}
