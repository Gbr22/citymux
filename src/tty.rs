use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
pub use crate::tty_windows::package::spawn_interactive_process;

#[cfg(unix)]
pub use crate::tty_unix::package::spawn_interactive_process;

#[derive(Serialize, Deserialize, Debug)]
pub struct TtyParameters {
    pub executable: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}
