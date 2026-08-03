use std::{collections::BTreeMap, env, fmt, fs, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Error, Result, limits::DEFAULT_COMMENTS_LIMIT};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

impl FromStr for OutputFormat {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            other => Err(Error::InvalidConfigKey(format!("output={other}"))),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => f.write_str("human"),
            Self::Json => f.write_str("json"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostConfig {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    pub default_hostname: Option<String>,
    #[serde(default)]
    pub hosts: BTreeMap<String, HostConfig>,
    pub ca_bundle: Option<String>,
    pub output: Option<OutputFormat>,
    pub comments_limit: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct CliConfigOverrides {
    pub hostname: Option<String>,
    pub token: Option<String>,
    pub ca_bundle: Option<String>,
    pub output: Option<OutputFormat>,
    pub comments_limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub hostname: String,
    pub token: Option<String>,
    pub api_base: String,
    pub ca_bundle: Option<String>,
    pub output: OutputFormat,
    pub comments_limit: u32,
}

impl EffectiveConfig {
    pub fn bearer_token(&self) -> Result<&str> {
        self.token.as_deref().ok_or(Error::MissingToken)
    }

    pub fn card_url(&self, card_id: u64) -> String {
        format!("https://{}/{card_id}", self.hostname)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthState {
    pub hostname: String,
    pub api_base: String,
    pub is_default: bool,
    pub has_token: bool,
    pub token_preview: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub config_file: PathBuf,
}

impl ConfigPaths {
    pub fn default_path() -> PathBuf {
        if let Ok(path) = env::var("KTEN_CONFIG") {
            return PathBuf::from(path);
        }
        if let Ok(path) = env::var("KTEN_CONFIG_DIR") {
            return PathBuf::from(path).join("config.toml");
        }

        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("kten")
                .join("config.toml")
        }

        #[cfg(not(target_os = "macos"))]
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kten")
            .join("config.toml")
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self {
            config_file: Self::default_path(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub paths: ConfigPaths,
    pub file: ConfigFile,
}

impl Config {
    pub fn load(paths: ConfigPaths) -> Result<Self> {
        let file = match fs::read_to_string(&paths.config_file) {
            Ok(contents) => toml::from_str(&contents).map_err(|source| Error::ConfigParse {
                path: paths.config_file.display().to_string(),
                source,
            })?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => ConfigFile::default(),
            Err(source) => {
                return Err(Error::ConfigIo {
                    path: paths.config_file.display().to_string(),
                    source,
                });
            }
        };
        Ok(Self { paths, file })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.paths.config_file.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::ConfigIo {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let serialized = toml::to_string_pretty(&self.file)?;
        fs::write(&self.paths.config_file, serialized).map_err(|source| Error::ConfigIo {
            path: self.paths.config_file.display().to_string(),
            source,
        })
    }

    pub fn effective(&self, cli: CliConfigOverrides) -> Result<EffectiveConfig> {
        let env_hostname = env::var("KTEN_HOSTNAME").ok();
        let env_token = env::var("KTEN_TOKEN").ok();
        let env_ca_bundle = env::var("KTEN_CA_BUNDLE").ok();
        let hostname = cli
            .hostname
            .or(env_hostname)
            .or_else(|| self.file.default_hostname.clone())
            .ok_or(Error::MissingHostname)?;
        validate_hostname(&hostname)?;

        let host_token = self
            .file
            .hosts
            .get(&hostname)
            .and_then(|host| host.token.clone());
        let token = cli.token.or(env_token).or(host_token);
        let ca_bundle = cli
            .ca_bundle
            .or(env_ca_bundle)
            .or_else(|| self.file.ca_bundle.clone());
        let output = cli
            .output
            .or(self.file.output)
            .unwrap_or(OutputFormat::Human);
        let comments_limit = cli
            .comments_limit
            .or(self.file.comments_limit)
            .unwrap_or(DEFAULT_COMMENTS_LIMIT);
        Ok(EffectiveConfig {
            api_base: default_api_base(&hostname),
            hostname,
            token,
            ca_bundle,
            output,
            comments_limit,
        })
    }

    pub fn login(&mut self, hostname: String, token: String) -> Result<()> {
        validate_hostname(&hostname)?;
        self.file
            .hosts
            .insert(hostname.clone(), HostConfig { token: Some(token) });
        self.file.default_hostname = Some(hostname);
        Ok(())
    }

    pub fn logout(&mut self, hostname: Option<String>) -> Result<()> {
        let target = match hostname {
            Some(value) => value,
            None => self
                .file
                .default_hostname
                .clone()
                .ok_or(Error::MissingHostname)?,
        };
        self.file.hosts.remove(&target);

        if self.file.default_hostname.as_deref() == Some(target.as_str()) {
            match self.file.hosts.len() {
                1 => {
                    self.file.default_hostname = self.file.hosts.keys().next().cloned();
                }
                _ => self.file.default_hostname = None,
            }
        }
        Ok(())
    }

    pub fn set_default_hostname(&mut self, hostname: String) -> Result<()> {
        validate_hostname(&hostname)?;
        if !self.file.hosts.contains_key(&hostname) {
            return Err(Error::InvalidConfigKey(format!(
                "unknown hostname: {hostname}; run auth login first"
            )));
        }
        self.file.default_hostname = Some(hostname);
        Ok(())
    }

    pub fn auth_state_for(&self, hostname: &str, show_token: bool) -> Result<AuthState> {
        validate_hostname(hostname)?;
        let token = self
            .file
            .hosts
            .get(hostname)
            .and_then(|host| host.token.clone());
        Ok(AuthState {
            hostname: hostname.to_string(),
            api_base: default_api_base(hostname),
            is_default: self.file.default_hostname.as_deref() == Some(hostname),
            has_token: token.is_some(),
            token_preview: token.map(|value| {
                if show_token {
                    value
                } else {
                    redact_token(&value)
                }
            }),
        })
    }

    pub fn auth_state_all(&self, show_token: bool) -> Vec<AuthState> {
        self.file
            .hosts
            .keys()
            .filter_map(|hostname| self.auth_state_for(hostname, show_token).ok())
            .collect()
    }

    pub fn set(&mut self, key: EditableConfigKey, value: String) -> Result<()> {
        match key {
            EditableConfigKey::DefaultHostname => self.set_default_hostname(value)?,
            EditableConfigKey::Output => self.file.output = Some(value.parse()?),
            EditableConfigKey::CaBundle => self.file.ca_bundle = Some(value),
            EditableConfigKey::CommentsLimit => {
                self.file.comments_limit = Some(value.parse().map_err(|_| {
                    Error::InvalidConfigKey("comments_limit must be an integer".to_string())
                })?);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableConfigKey {
    DefaultHostname,
    Output,
    CaBundle,
    CommentsLimit,
}

impl FromStr for EditableConfigKey {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "default_hostname" => Ok(Self::DefaultHostname),
            "output" => Ok(Self::Output),
            "ca_bundle" => Ok(Self::CaBundle),
            "comments_limit" => Ok(Self::CommentsLimit),
            "token" => Err(Error::SecretConfigKey(value.to_string())),
            other => Err(Error::InvalidConfigKey(other.to_string())),
        }
    }
}

pub fn default_api_base(hostname: &str) -> String {
    format!("https://{hostname}/api/latest")
}

pub fn redact_token(token: &str) -> String {
    let visible = token.chars().take(4).collect::<String>();
    format!("{visible}...")
}

pub fn validate_hostname(hostname: &str) -> Result<()> {
    if hostname.trim().is_empty() || hostname != hostname.trim() {
        return Err(Error::InvalidHostname(hostname.to_string()));
    }
    let url = Url::parse(&format!("https://{hostname}"))
        .map_err(|_| Error::InvalidHostname(hostname.to_string()))?;
    if url.host_str() != Some(hostname)
        || url.scheme() != "https"
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(Error::InvalidHostname(hostname.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn base_config() -> Config {
        Config {
            paths: ConfigPaths::default(),
            file: ConfigFile::default(),
        }
    }

    fn clear_config_env() {
        unsafe {
            env::remove_var("KTEN_HOSTNAME");
            env::remove_var("KTEN_TOKEN");
            env::remove_var("KTEN_CA_BUNDLE");
            env::remove_var("KTEN_API_BASE");
            env::remove_var("KTEN_CONFIG");
            env::remove_var("KTEN_CONFIG_DIR");
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn default_api_base_uses_full_hostname() {
        assert_eq!(
            default_api_base("company.kaiten.ru"),
            "https://company.kaiten.ru/api/latest"
        );
    }

    #[test]
    fn auth_state_does_not_expose_token_by_default() {
        let mut config = base_config();
        config.file.default_hostname = Some("company.kaiten.ru".to_string());
        config.file.hosts.insert(
            "company.kaiten.ru".to_string(),
            HostConfig {
                token: Some("secret-token".to_string()),
            },
        );
        assert_eq!(
            config
                .auth_state_for("company.kaiten.ru", false)
                .unwrap()
                .token_preview,
            Some("secr...".to_string())
        );
    }

    #[test]
    fn card_url_matches_plan() {
        let effective = EffectiveConfig {
            hostname: "company.kaiten.ru".to_string(),
            token: None,
            api_base: String::new(),
            ca_bundle: None,
            output: OutputFormat::Human,
            comments_limit: 10,
        };
        assert_eq!(effective.card_url(123), "https://company.kaiten.ru/123");
    }

    #[test]
    fn ca_bundle_is_editable_and_token_is_secret_key() {
        assert!("ca_bundle".parse::<EditableConfigKey>().is_ok());
        assert!(matches!(
            "token".parse::<EditableConfigKey>(),
            Err(Error::SecretConfigKey(_))
        ));
    }

    #[test]
    fn effective_uses_cli_env_config_precedence() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        let mut config = base_config();
        config.file.default_hostname = Some("config.kaiten.ru".to_string());
        config.file.ca_bundle = Some("/tmp/config-ca.pem".to_string());
        config.file.hosts.insert(
            "config.kaiten.ru".to_string(),
            HostConfig {
                token: Some("config-token".to_string()),
            },
        );
        unsafe {
            env::set_var("KTEN_HOSTNAME", "env.kaiten.ru");
            env::set_var("KTEN_TOKEN", "env-token");
            env::set_var("KTEN_CA_BUNDLE", "/tmp/env-ca.pem");
        }
        config.file.hosts.insert(
            "env.kaiten.ru".to_string(),
            HostConfig {
                token: Some("env-host-token".to_string()),
            },
        );
        config.file.hosts.insert(
            "cli.kaiten.ru".to_string(),
            HostConfig {
                token: Some("cli-host-token".to_string()),
            },
        );

        let effective = config
            .effective(CliConfigOverrides {
                hostname: Some("cli.kaiten.ru".to_string()),
                token: Some("cli-token".to_string()),
                ca_bundle: Some("/tmp/cli-ca.pem".to_string()),
                output: None,
                comments_limit: None,
            })
            .unwrap();
        assert_eq!(effective.hostname, "cli.kaiten.ru");
        assert_eq!(effective.token.as_deref(), Some("cli-token"));
        assert_eq!(effective.ca_bundle.as_deref(), Some("/tmp/cli-ca.pem"));
        assert_eq!(effective.api_base, "https://cli.kaiten.ru/api/latest");

        clear_config_env();
    }

    #[test]
    fn effective_falls_back_to_host_token() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        let mut config = base_config();
        config.file.default_hostname = Some("company.kaiten.ru".to_string());
        config.file.hosts.insert(
            "company.kaiten.ru".to_string(),
            HostConfig {
                token: Some("stored-token".to_string()),
            },
        );

        let effective = config.effective(CliConfigOverrides::default()).unwrap();
        assert_eq!(effective.token.as_deref(), Some("stored-token"));
        clear_config_env();
    }

    #[test]
    fn env_overrides_config_for_ca_bundle() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        let mut config = base_config();
        config.file.default_hostname = Some("company.kaiten.ru".to_string());
        config.file.ca_bundle = Some("/tmp/from-config.pem".to_string());

        unsafe { env::set_var("KTEN_CA_BUNDLE", "/tmp/from-env.pem") };
        let from_env = config.effective(CliConfigOverrides::default()).unwrap();
        assert_eq!(from_env.ca_bundle.as_deref(), Some("/tmp/from-env.pem"));
        clear_config_env();
    }

    #[test]
    fn ignores_kten_api_base_env_var() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        let mut config = base_config();
        config.file.default_hostname = Some("company.kaiten.ru".to_string());

        unsafe { env::set_var("KTEN_API_BASE", "https://from-env/api/latest") };
        let effective = config.effective(CliConfigOverrides::default()).unwrap();
        assert_eq!(effective.api_base, "https://company.kaiten.ru/api/latest");
        clear_config_env();
    }

    #[test]
    fn set_default_hostname_rejects_unknown_host() {
        let mut config = base_config();
        let err = config
            .set_default_hostname("unknown.kaiten.ru".to_string())
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfigKey(_)));
    }

    #[test]
    fn logout_reassigns_or_clears_default() {
        let mut config = base_config();
        config.file.default_hostname = Some("a.kaiten.ru".to_string());
        config.file.hosts.insert(
            "a.kaiten.ru".to_string(),
            HostConfig {
                token: Some("a".to_string()),
            },
        );
        config.file.hosts.insert(
            "b.kaiten.ru".to_string(),
            HostConfig {
                token: Some("b".to_string()),
            },
        );
        config.logout(Some("a.kaiten.ru".to_string())).unwrap();
        assert_eq!(config.file.default_hostname.as_deref(), Some("b.kaiten.ru"));

        config.file.hosts.insert(
            "c.kaiten.ru".to_string(),
            HostConfig {
                token: Some("c".to_string()),
            },
        );
        config.file.hosts.insert(
            "d.kaiten.ru".to_string(),
            HostConfig {
                token: Some("d".to_string()),
            },
        );
        config.file.default_hostname = Some("b.kaiten.ru".to_string());
        config.logout(Some("b.kaiten.ru".to_string())).unwrap();
        assert_eq!(config.file.default_hostname, None);
    }

    #[test]
    fn validate_hostname_rejects_scheme_and_path() {
        assert!(validate_hostname("https://company.kaiten.ru").is_err());
        assert!(validate_hostname("company.kaiten.ru/path").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn default_path_on_macos_uses_home_dot_config() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();

        let expected = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("kten")
            .join("config.toml");

        assert_eq!(ConfigPaths::default_path(), expected);
        clear_config_env();
    }

    #[test]
    fn config_dir_env_overrides_default_path() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        unsafe { env::set_var("KTEN_CONFIG_DIR", "/tmp/kten-config-dir") };

        assert_eq!(
            ConfigPaths::default_path(),
            PathBuf::from("/tmp/kten-config-dir").join("config.toml")
        );

        clear_config_env();
    }

    #[test]
    fn config_file_env_overrides_config_dir_env() {
        let _guard = env_lock().lock().unwrap();
        clear_config_env();
        unsafe { env::set_var("KTEN_CONFIG", "/tmp/kten-config-file.toml") };
        unsafe { env::set_var("KTEN_CONFIG_DIR", "/tmp/kten-config-dir") };

        assert_eq!(
            ConfigPaths::default_path(),
            PathBuf::from("/tmp/kten-config-file.toml")
        );

        clear_config_env();
    }
}
