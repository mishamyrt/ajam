mod connect;
mod events;
mod render;
mod navigation;

use ajazz_sdk::AsyncAjazz;
use std::collections::HashMap;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use tokio::sync::RwLock;

use ajam_profile::{Page, Profile};

pub use connect::StateConnect;
pub use events::StateEventsHandler;

pub const DEFAULT_PROFILE: &str = "common";
pub const DEFAULT_PAGE: &str = "main";

#[derive(Debug, Clone)]
struct NavigationState {
    pub profile: String,
    pub page: String,
}

#[derive(Clone)]
pub(crate) struct State {
    dev: Arc<RwLock<Option<AsyncAjazz>>>,
    profiles: Arc<RwLock<HashMap<String, Profile>>>,
    navigation: Arc<RwLock<NavigationState>>,
    brightness: Arc<AtomicU8>,
}

impl State {
    pub fn with_profiles(profiles: HashMap<String, Profile>) -> Self {
        Self {
            dev: Arc::new(RwLock::new(None)),
            profiles: Arc::new(RwLock::new(profiles)),
            navigation: Arc::new(RwLock::new(NavigationState {
                profile: DEFAULT_PROFILE.to_string(),
                page: DEFAULT_PAGE.to_string(),
            })),
            brightness: Arc::new(AtomicU8::new(100)),
        }
    }

    async fn get_page(&self, profile: &str, page: &str) -> Option<(Profile, Page)> {
        let profiles_guard = self.profiles.read().await;

        let profile = profiles_guard.get(profile)?;
        let page = profile.pages.get(page)?;

        Some((profile.clone(), page.clone()))
    }
}
