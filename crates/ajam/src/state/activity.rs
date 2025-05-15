use std::sync::mpsc;
use colored::Colorize;

use ajam_activity::Event;

use crate::{print_debug, print_error};


use super::{
    navigation::{Navigator, DEFAULT_PROFILE},
    State,
};

pub(crate) trait ActivityHandler {
    async fn listen_activity_events(&self, rx: mpsc::Receiver<Event>);
}

impl ActivityHandler for State {
    async fn listen_activity_events(&self, rx: mpsc::Receiver<Event>) {
        while let Ok(event) = rx.recv() {
            match event {
                Event::AppChange(bundle_id) => {
                    let profile = {
                        let navigation_guard = self.navigation.read().await;
                        navigation_guard.profile.clone()
                    };

                    if profile == bundle_id {
                        continue;
                    }

                    {
                        let mut active_profile_guard = self.active_profile.write().await;
                        *active_profile_guard = DEFAULT_PROFILE.to_string();
                    }

                    if let Err(e) = self.navigate_to_profile_or_default(&bundle_id).await {
                        print_error!("error navigating to profile: {:?}", e);
                    }
                }
                Event::AudioOutputChange(device_name) => {
                    print_debug!("Audio output changed: {}", device_name);
                }
                Event::AudioInputChange(device_name) => {
                    print_debug!("Audio input changed: {}", device_name);
                }
            }
        }
    }
}
