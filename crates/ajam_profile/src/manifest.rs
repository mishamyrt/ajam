use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Encoder {
    pub plus: Action,
    pub minus: Action,
    pub click: Option<Action>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub pages_order: Vec<String>,
    pub pages: HashMap<String, Page>,
    pub encoders: HashMap<char, Encoder>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Page {
    #[serde(flatten)]
    pub buttons: HashMap<char, ButtonConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ButtonConfig {
    pub image: String,
    pub action: Action,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Action {
    Keys { keys: String },
    Command { command: String },
    Navigate { navigate: String },
}

impl Manifest {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, crate::ProfileError> {
        let data = fs::read_to_string(path)?;
        let manifest: Manifest = serde_yaml::from_str(&data)?;
        Ok(manifest)
    }
}

