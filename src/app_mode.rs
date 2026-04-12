use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

pub enum AppMode {
    Setup,
    Normal(Arc<AppState>),
}

pub type SharedAppMode = Arc<RwLock<AppMode>>;

pub fn initial_mode(state: Option<AppState>) -> SharedAppMode {
    match state {
        Some(s) => Arc::new(RwLock::new(AppMode::Normal(Arc::new(s)))),
        None => Arc::new(RwLock::new(AppMode::Setup)),
    }
}
