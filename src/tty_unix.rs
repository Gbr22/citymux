#[cfg(unix)]
pub mod package {
    use std::collections::HashMap;
    use renterm::vector::Vector2;
    use crate::process::ProcessData;

    pub async fn spawn_interactive_process(
        program_to_spawn: &str,
        env: &HashMap<String, String>,
        args: &[String],
        size: Vector2,
    ) -> anyhow::Result<ProcessData> {
        Err(anyhow::anyhow!("Not implemented"))
    }
}