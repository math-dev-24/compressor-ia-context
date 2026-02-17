use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Global + per-project configuration.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    /// Maximum lines before truncation.
    pub max_lines: usize,
    /// Maximum characters per line.
    pub max_line_len: usize,
    /// Show timing footer on each command.
    pub show_footer: bool,
    /// Directories to skip in `cx ls`.
    pub ls_skip: Vec<String>,
    /// Max depth for `cx ls`.
    pub ls_max_depth: usize,
    /// Max entries for `cx ls`.
    pub ls_max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_lines: 150,
            max_line_len: 300,
            show_footer: true,
            ls_skip: vec![
                "target".into(),
                "node_modules".into(),
                ".git".into(),
                "__pycache__".into(),
                ".DS_Store".into(),
                "dist".into(),
                "build".into(),
                ".next".into(),
                ".cache".into(),
                "coverage".into(),
                ".venv".into(),
                "venv".into(),
            ],
            ls_max_depth: 4,
            ls_max_entries: 200,
        }
    }
}

impl Config {
    /// Load config with priority: .cx.toml (project) > ~/.config/cx/config.toml (global) > defaults.
    pub fn load() -> Self {
        let mut config = Self::default();

        // 1. Global config
        if let Some(path) = global_config_path()
            && let Some(global) = load_file(&path)
        {
            config = merge(config, global);
        }

        // 2. Project config (overrides global)
        if let Some(project) = load_file(Path::new(".cx.toml")) {
            config = merge(config, project);
        }

        config
    }

    /// Generate a default config file content.
    pub fn default_toml() -> &'static str {
        r#"# cx-proxy configuration
# Place in ~/.config/cx/config.toml (global) or .cx.toml (per-project)

# Truncation limits
max_lines = 150
max_line_len = 300

# Show timing footer after each command
show_footer = true

# Directories to skip in `cx ls`
ls_skip = [
    "target", "node_modules", ".git", "__pycache__",
    ".DS_Store", "dist", "build", ".next", ".cache",
    "coverage", ".venv", "venv",
]

# Tree listing limits
ls_max_depth = 4
ls_max_entries = 200
"#
    }
}

/// Partial config for TOML deserialization (all fields optional).
#[derive(Debug, Deserialize)]
struct PartialConfig {
    max_lines: Option<usize>,
    max_line_len: Option<usize>,
    show_footer: Option<bool>,
    ls_skip: Option<Vec<String>>,
    ls_max_depth: Option<usize>,
    ls_max_entries: Option<usize>,
}

fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("cx").join("config.toml"))
}

fn load_file(path: &Path) -> Option<PartialConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn merge(base: Config, partial: PartialConfig) -> Config {
    Config {
        max_lines: partial.max_lines.unwrap_or(base.max_lines),
        max_line_len: partial.max_line_len.unwrap_or(base.max_line_len),
        show_footer: partial.show_footer.unwrap_or(base.show_footer),
        ls_skip: partial.ls_skip.unwrap_or(base.ls_skip),
        ls_max_depth: partial.ls_max_depth.unwrap_or(base.ls_max_depth),
        ls_max_entries: partial.ls_max_entries.unwrap_or(base.ls_max_entries),
    }
}

/// Detect the project type based on files present in the current directory.
pub fn detect_project() -> Vec<ProjectType> {
    let mut types = Vec::new();

    if Path::new("Cargo.toml").exists() {
        types.push(ProjectType::Rust);
    }
    if Path::new("package.json").exists() {
        types.push(ProjectType::Node);
    }
    if Path::new("pyproject.toml").exists()
        || Path::new("setup.py").exists()
        || Path::new("requirements.txt").exists()
    {
        types.push(ProjectType::Python);
    }
    if Path::new("go.mod").exists() {
        types.push(ProjectType::Go);
    }
    if Path::new("Dockerfile").exists()
        || Path::new("docker-compose.yml").exists()
        || Path::new("compose.yml").exists()
    {
        types.push(ProjectType::Docker);
    }
    if Path::new("Makefile").exists() {
        types.push(ProjectType::Make);
    }

    types
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Docker,
    Make,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::Node => write!(f, "node"),
            Self::Python => write!(f, "python"),
            Self::Go => write!(f, "go"),
            Self::Docker => write!(f, "docker"),
            Self::Make => write!(f, "make"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.max_lines, 150);
        assert_eq!(config.max_line_len, 300);
        assert!(config.show_footer);
        assert!(config.ls_skip.contains(&"target".to_string()));
    }

    #[test]
    fn test_merge_partial() {
        let base = Config::default();
        let partial = PartialConfig {
            max_lines: Some(50),
            max_line_len: None,
            show_footer: Some(false),
            ls_skip: None,
            ls_max_depth: None,
            ls_max_entries: None,
        };
        let merged = merge(base, partial);
        assert_eq!(merged.max_lines, 50);
        assert_eq!(merged.max_line_len, 300); // kept default
        assert!(!merged.show_footer);
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
max_lines = 100
show_footer = false
"#;
        let partial: PartialConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(partial.max_lines, Some(100));
        assert_eq!(partial.show_footer, Some(false));
        assert!(partial.max_line_len.is_none());
    }

    #[test]
    fn test_default_toml_is_valid() {
        let toml_str = Config::default_toml();
        let result: Result<PartialConfig, _> = toml::from_str(toml_str);
        assert!(result.is_ok());
    }
}
