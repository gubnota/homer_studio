pub mod commands;
pub mod runtime;
pub mod services;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
pub struct AppState {
    pub project_write_lock: Arc<Mutex<()>>,
    pub jobs: services::jobs::JobStore,
    pub audio_assets: Arc<RwLock<HashMap<String, PathBuf>>>,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            project_write_lock: Arc::new(Mutex::new(())),
            jobs: services::jobs::JobStore::new(),
            audio_assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
