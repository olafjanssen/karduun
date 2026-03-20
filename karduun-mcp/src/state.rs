use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub repo_root: Arc<Mutex<PathBuf>>,
}

impl ServerState {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root: Arc::new(Mutex::new(repo_root)),
        }
    }
}
