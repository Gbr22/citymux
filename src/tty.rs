use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use crate::tty_windows::spawn_interactive_process;

#[derive(Serialize, Deserialize, Debug)]
pub struct TtyParameters {
    pub executable: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}
