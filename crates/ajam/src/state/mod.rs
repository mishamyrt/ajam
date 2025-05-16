mod activity;
mod connect;
mod events;
mod navigation;
mod render;

use ajazz_sdk::AsyncAjazz;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::{collections::HashMap, num::NonZero};
use tokio::sync::{Mutex, RwLock};

use ajam_profile::{ImageCache, Page, Profile};

pub(crate) use activity::ActivityHandler;
pub(crate) use connect::StateConnect;

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
    active_profile: Arc<RwLock<String>>,
    navigation: Arc<RwLock<NavigationState>>,
    brightness: Arc<AtomicU8>,
    image_cache: Arc<Mutex<ImageCache>>,
    audio_output_device: Arc<RwLock<String>>,
    audio_input_device: Arc<RwLock<String>>,
}

impl State {
    pub fn with_profiles(profiles: HashMap<String, Profile>) -> Self {
        Self {
            dev: Arc::new(RwLock::new(None)),
            profiles: Arc::new(RwLock::new(profiles)),
            active_profile: Arc::new(RwLock::new(DEFAULT_PROFILE.to_string())),
            navigation: Arc::new(RwLock::new(NavigationState {
                profile: DEFAULT_PROFILE.to_string(),
                page: DEFAULT_PAGE.to_string(),
            })),
            brightness: Arc::new(AtomicU8::new(100)),
            image_cache: Arc::new(Mutex::new(ImageCache::new(NonZero::new(120).unwrap()))),
            audio_output_device: Arc::new(RwLock::new(String::new())),
            audio_input_device: Arc::new(RwLock::new(String::new())),
        }
    }

    async fn get_page(&self, profile: &str, page: &str) -> Option<(Profile, Page)> {
        let profiles_guard = self.profiles.read().await;

        let profile = profiles_guard.get(profile)?;
        let page = profile.manifest.get_page(page)?;

        Some((profile.clone(), page.clone()))
    }

    async fn set_audio_output_device(&self, device: &str) {
        *self.audio_output_device.write().await = device.to_string();
    }

    async fn set_audio_input_device(&self, device: &str) {
        *self.audio_input_device.write().await = device.to_string();
    }
}
