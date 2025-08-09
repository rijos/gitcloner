use std::env;
use std::process;
use bcrypt::{hash, DEFAULT_COST};
use sqlx::{SqlitePool, Row};
use tokio;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:gitcloner.db".to_string());
    
    let pool = match SqlitePool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            process::exit(1);
        }
    };

    match args[1].as_str() {
        "add" => {
            if args.len() != 4 {
                eprintln!("Usage: {} add <username> <password>", args[0]);
                process::exit(1);
            }
            add_user(&pool, &args[2], &args[3]).await;
        }
        "remove" => {
            if args.len() != 3 {
                eprintln!("Usage: {} remove <username>", args[0]);
                process::exit(1);
            }
            remove_user(&pool, &args[2]).await;
        }
        "list" => {
            list_users(&pool).await;
        }
        "update" => {
            if args.len() != 4 {
                eprintln!("Usage: {} update <username> <new_password>", args[0]);
                process::exit(1);
            }
            update_user_password(&pool, &args[2], &args[3]).await;
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("GitCloner Admin Tool");
    println!();
    println!("USAGE:");
    println!("    gitc add <username> <password>     - Add or update user");
    println!("    gitc remove <username>             - Remove user");
    println!("    gitc update <username> <password>  - Update user password");
    println!("    gitc list                          - List all users");
    println!();
    println!("EXAMPLES:");
    println!("    gitc add admin admin123");
    println!("    gitc add john secret456");
    println!("    gitc remove admin");
    println!("    gitc update john newpassword789");
    println!();
    println!("ENVIRONMENT:");
    println!("    DATABASE_URL - Database connection string (default: sqlite:gitcloner.db)");
}

async fn add_user(pool: &SqlitePool, username: &str, password: &str) {
    let password_hash = match hash(password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Failed to hash password: {}", e);
            process::exit(1);
        }
    };

    let result = sqlx::query(
        "INSERT OR REPLACE INTO users (username, password_hash) VALUES (?, ?)"
    )
    .bind(username)
    .bind(&password_hash)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            println!("✓ User '{}' created/updated successfully", username);
            println!("  Username: {}", username);
            println!("  Password: {}", password);
        }
        Err(e) => {
            eprintln!("Failed to create user '{}': {}", username, e);
            process::exit(1);
        }
    }
}

async fn remove_user(pool: &SqlitePool, username: &str) {
    let result = sqlx::query("DELETE FROM users WHERE username = ?")
        .bind(username)
        .execute(pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                println!("✓ User '{}' removed successfully", username);
            } else {
                println!("! User '{}' not found", username);
            }
        }
        Err(e) => {
            eprintln!("Failed to remove user '{}': {}", username, e);
            process::exit(1);
        }
    }
}

async fn update_user_password(pool: &SqlitePool, username: &str, new_password: &str) {
    let password_hash = match hash(new_password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Failed to hash password: {}", e);
            process::exit(1);
        }
    };

    let result = sqlx::query("UPDATE users SET password_hash = ? WHERE username = ?")
        .bind(&password_hash)
        .bind(username)
        .execute(pool)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                println!("✓ Password for user '{}' updated successfully", username);
            } else {
                eprintln!("! User '{}' not found. Use 'add {} <password>' to create one.", username, username);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to update password for user '{}': {}", username, e);
            process::exit(1);
        }
    }
}

async fn list_users(pool: &SqlitePool) {
    let result = sqlx::query("SELECT username, created_at FROM users ORDER BY created_at")
        .fetch_all(pool)
        .await;

    match result {
        Ok(rows) => {
            if rows.is_empty() {
                println!("No users found");
            } else {
                println!("Users:");
                for row in rows {
                    let username: String = row.get("username");
                    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
                    println!("  {} (created: {})", username, created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to list users: {}", e);
            process::exit(1);
        }
    }
}
