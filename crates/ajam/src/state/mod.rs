mod render;
mod events;
mod connect;

use ajazz_sdk::AsyncAjazz;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::Path;

use ajam_profile::{open_profiles, Profile, ProfileError};

pub use events::StateEventsHandler;
pub use connect::StateConnect;


pub const DEFAULT_APP: &str = "common";
pub const DEFAULT_PAGE: &str = "main";
pub const MANIFEST: &str = "manifest.json";

#[derive(Debug)]
pub struct StateData {
    pub active_app: String,
    pub active_page: String,
    pub profiles: HashMap<String, Profile>,
}

impl StateData {
    pub fn new() -> Self {
        Self {
            active_app: DEFAULT_APP.to_string(),
            active_page: DEFAULT_PAGE.to_string(),
            profiles: HashMap::new(),
        }
    }

    pub fn from_dir(dir: &Path) -> Result<Self, ProfileError> {
        let profiles = open_profiles(dir)?;
        Ok(Self {
            active_app: DEFAULT_APP.to_string(),
            active_page: DEFAULT_PAGE.to_string(),
            profiles,
        })
    }
}

#[derive(Clone)]
pub(crate) struct State {
    dev: Arc<RwLock<Option<AsyncAjazz>>>,
    data: Arc<RwLock<StateData>>,
    brightness: Arc<AtomicU8>,
}

impl State {
    pub fn from_data(data: StateData) -> Self {
        Self {
            dev: Arc::new(RwLock::new(None)),
            data: Arc::new(RwLock::new(data)),
            brightness: Arc::new(AtomicU8::new(100)),
        }
    }
}
