mod manifest;
mod profile;
mod materialized_page;

pub use profile::{Profile, Page, Action, EncoderActions, open_profiles};
pub use manifest::{Manifest, ButtonConfig};
pub use materialized_page::{MaterializedPage, ImageLoader, MaterializedPageError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("Profile not found")]
    ProfileNotFound,

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JSONError(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    YAMLError(#[from] serde_yaml::Error),

    #[error("Invalid key combo: {0}")]
    InvalidKeyCombo(String),

    #[error("Invalid manifest")]
    InvalidManifest,

    #[error("Invalid app id: {0}")]
    InvalidAppId(String),
    
    #[error("Manifest file not found at {0}")]
    ManifestFileNotFound(String),
}