mod manifest;
mod profile;

pub use profile::{Profile, Page, Action};
pub use manifest::{Manifest, ButtonConfig};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("Profile not found")]
    ProfileNotFound,

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JSONError(#[from] serde_json::Error),

    #[error("Invalid manifest")]
    InvalidManifest,
}