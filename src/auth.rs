use bcrypt::verify;
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthManager {
    sessions: Arc<RwLock<HashMap<String, String>>>, // token -> username
}

impl AuthManager {
    pub fn new() -> Self {
        AuthManager {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        Ok(verify(password, hash)?)
    }

    pub async fn create_session(&self, username: &str) -> String {
        let token = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(token.clone(), username.to_string());
        token
    }

    pub async fn validate_session(&self, token: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions.get(token).cloned()
    }

    pub async fn remove_session(&self, token: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token);
    }
}
