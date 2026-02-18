use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

use crate::project::model::ProjectFile;

pub struct LoadedProject {
    pub project: ProjectFile,
    pub json_path: PathBuf,
    pub project_dir: PathBuf,
    pub dirty: bool,
}

pub struct AppState {
    pub inner: Mutex<Option<LoadedProject>>,
    pub save_notify: Notify,
    pub task_notify: Notify,
    pub cancel_flags: Mutex<std::collections::HashSet<String>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(None),
            save_notify: Notify::new(),
            task_notify: Notify::new(),
            cancel_flags: Mutex::new(std::collections::HashSet::new()),
        })
    }
}
