use anyhow::Result;
use sqlx::{SqlitePool, Row};
use std::path::Path;
use std::fs;
use crate::models::{User, Repository};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        println!("Connecting to database: {}", database_url);
        
        // Handle SQLite database file creation
        if database_url.starts_with("sqlite:") {
            let db_path = &database_url[7..]; // Remove "sqlite:" prefix
            
            // Create the database file if it doesn't exist
            if !Path::new(db_path).exists() {
                println!("Database file doesn't exist, creating: {}", db_path);
                
                // Create parent directories if needed
                if let Some(parent) = Path::new(db_path).parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Create empty database file
                fs::File::create(db_path)?;
            }
        }
        
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Database { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        let migration_sql = include_str!("../migrations/001_initial.sql");
        sqlx::query(migration_sql).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, created_at FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn add_repository(&self, url: &str, name: &str, local_path: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO repositories (url, name, local_path, status) VALUES (?, ?, ?, 'pending')"
        )
        .bind(url)
        .bind(name)
        .bind(local_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_all_repositories(&self) -> Result<Vec<Repository>> {
        let rows = sqlx::query(
            "SELECT id, url, name, local_path, last_synced, created_at, status FROM repositories ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut repositories = Vec::new();
        for row in rows {
            repositories.push(Repository {
                id: row.get("id"),
                url: row.get("url"),
                name: row.get("name"),
                local_path: row.get("local_path"),
                last_synced: row.get("last_synced"),
                created_at: row.get("created_at"),
                status: row.get("status"),
            });
        }
        Ok(repositories)
    }

    pub async fn remove_repository(&self, url: &str) -> Result<()> {
        sqlx::query("DELETE FROM repositories WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_repository_status(&self, url: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE repositories SET status = ? WHERE url = ?")
            .bind(status)
            .bind(url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_last_synced(&self, url: &str) -> Result<()> {
        sqlx::query("UPDATE repositories SET last_synced = CURRENT_TIMESTAMP WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
