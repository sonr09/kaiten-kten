use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing Kaiten hostname; set KTEN_HOSTNAME, pass --hostname, or run auth login")]
    MissingHostname,
    #[error("missing Kaiten token; set KTEN_TOKEN or run auth login")]
    MissingToken,
    #[error("invalid hostname: {0}")]
    InvalidHostname(String),
    #[error("invalid config key: {0}")]
    InvalidConfigKey(String),
    #[error("secret config key cannot be set with config set: {0}")]
    SecretConfigKey(String),
    #[error("invalid limit {value}; maximum is {max}")]
    LimitTooHigh { value: u32, max: u32 },
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to read CA bundle at {path}: {source}")]
    CaBundleRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("CA bundle at {path} is empty")]
    CaBundleEmpty { path: String },
    #[error("failed to parse CA bundle at {path}: {details}")]
    CaBundleParse { path: String, details: String },
    #[error("Kaiten API returned {status}: {message}")]
    Api { status: u16, message: String },
    #[error("config I/O failed at {path}: {source}")]
    ConfigIo {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("config parse failed at {path}: {source}")]
    ConfigParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("config serialization failed: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}
