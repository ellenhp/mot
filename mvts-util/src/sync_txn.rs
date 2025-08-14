use std::sync::{Mutex, OnceLock};

use tokio::task::JoinHandle;

pub static JOIN_HANDLES: OnceLock<Mutex<Vec<JoinHandle<()>>>> = OnceLock::new();
