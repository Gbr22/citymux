use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Config {
    pub default_shell: String,
}

pub fn get_default_config() -> Config {
    let os_default_shell = if cfg!(target_os = "windows") {
        "cmd.exe".to_string()
    } else {
        "bash".to_string()
    };
    let default_shell = std::env::var("SHELL").ok().unwrap_or(os_default_shell);

    Config { default_shell }
}

fn get_xdg_config_dir() -> Option<PathBuf> {
    let dir = std::env::var("XDG_CONFIG_HOME").ok()?;

    let path = PathBuf::from(dir);
    if path.exists() && path.is_dir() {
        return Some(path);
    }

    None
}

fn get_home_config_dir() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let path = home_dir.join(".config");

    Some(path)
}

fn get_config_dir() -> Option<PathBuf> {
    get_xdg_config_dir().or_else(|| get_home_config_dir())
}

fn get_config_optional() -> Option<Config> {
    let mut config = get_default_config();
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("citymux").join("config.kdl");
    let contents = std::fs::read_to_string(config_file).ok()?;
    let document = kdl::KdlDocument::parse_v2(&contents).ok()?;
    let shell_node = document
        .nodes()
        .iter()
        .find(|e| e.name().to_string() == "default_shell");
    if let Some(shell_node) = shell_node {
        let shell = shell_node.entries().first();
        if let Some(shell) = shell {
            let shell = shell.value().as_string();
            if let Some(shell) = shell {
                config.default_shell = shell.to_string();
            }
        };
    };

    Some(config)
}

pub fn get_config() -> Config {
    get_config_optional().unwrap_or(get_default_config())
}
