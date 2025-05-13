use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc, Arc,
    },
};

use ajam_events::ActivityEvent;
use ajam_keypress::{Performer, DECK_BRIGHTNESS_DOWN, DECK_BRIGHTNESS_UP};
use ajam_profile::{Action, Page, Profile, ProfileError};
use ajazz_sdk::{asynchronous::AsyncDeviceStateReader, AsyncAjazz, DeviceStateUpdate};
use ajazz_sdk::{list_devices, new_hidapi};
use colored::Colorize;
use image::open;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::{print_debug, print_error, print_info, print_warning, shell::run_command};

const MANIFEST: &str = "manifest.json";
const DEFAULT_APP: &str = "common";
const DEFAULT_PAGE: &str = "main";

#[derive(Debug)]
pub(crate) struct StateData {
    active_app: String,
    active_page: String,

    profiles: HashMap<String, Profile>,
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
        let mut state = Self::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let manifest_path = path.join(MANIFEST);
                if !manifest_path.exists() {
                    continue;
                }

                let profile = Profile::from_file(&manifest_path, 6)?;
                state.profiles.insert(profile.app_id.clone(), profile);
            }
        }

        Ok(state)
    }

    pub fn update_active_app(&mut self, bundle_id: &str) {
        if self.active_app == bundle_id {
            return;
        }

        print_info!("target_page_id: {}", bundle_id);
        if self.profiles.contains_key(bundle_id) {
            self.active_app = bundle_id.to_string();
        } else {
            self.active_app = DEFAULT_APP.to_string();
        }
        print_info!("active_app: {}", self.active_app);
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

    async fn render_page(&self, page: &Page) -> Option<()> {
        print_debug!("render_page: {:?}", page);
        let images_to_render = page
            .buttons
            .iter()
            .enumerate()
            .filter_map(|(i, button)| {
                if let Some(button) = button {
                    match open(button.image.clone()) {
                        Ok(image) => Some((i as u8, image)),
                        Err(e) => {
                            print_error!(
                                "Error loading image: {} {:?}",
                                button.image.to_string_lossy(),
                                e
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        self.render_page_with_images(&images_to_render).await
    }

    async fn render_page_with_images(
        &self,
        images_to_render: &[(u8, image::DynamicImage)],
    ) -> Option<()> {
        let dev = {
            let dev_guard = self.dev.read().await;
            match &*dev_guard {
                Some(dev) => dev.clone(),
                None => return None,
            }
        };

        let brightness = self.brightness.load(Ordering::Relaxed);

        dev.set_brightness(brightness).await.unwrap();
        dev.clear_all_button_images().await.unwrap();

        for &(i, ref image) in images_to_render {
            dev.set_button_image(i, image.clone()).await.unwrap();
        }

        for i in 0..6 {
            if !images_to_render.iter().any(|(btn_i, _)| *btn_i == i) {
                dev.clear_button_image(i).await.unwrap();
            }
        }

        print_debug!("render_end");
        dev.flush().await.unwrap();
        print_debug!("flush_end");

        Some(())
    }

    async fn update_brightness(&self) {
        let brightness = self.brightness.load(Ordering::Relaxed);
        let dev = {
            let dev_guard = self.dev.read().await;
            dev_guard.as_ref().unwrap().clone()
        };
        dev.set_brightness(brightness).await.unwrap();
    }

    pub(crate) async fn listen_os_events(self, rx: mpsc::Receiver<ActivityEvent>) {
        let mut last_bundle_id: Option<String> = None;

        while let Ok(event) = rx.recv() {
            match event {
                ActivityEvent::AppChange(bundle_id) => {
                    if last_bundle_id.as_ref() != Some(&bundle_id) {
                        print_debug!("handle_os_events: app change");

                        let (page, _profile_path) = {
                            let mut data_guard = self.data.write().await;
                            print_debug!("handle_os_events: update_active_app");
                            data_guard.update_active_app(&bundle_id);
                            last_bundle_id = Some(bundle_id.clone());

                            let Some(profile) = data_guard.profiles.get(&data_guard.active_app)
                            else {
                                print_warning!("no profile for active app");
                                continue;
                            };

                            let Some(page) = profile.pages.get(&data_guard.active_page) else {
                                print_warning!("no page for active page");
                                continue;
                            };

                            (page.clone(), profile.path.clone())
                        };

                        print_debug!("event handled");

                        let has_device = {
                            let dev_guard = self.dev.read().await;
                            dev_guard.is_some()
                        };

                        if has_device {
                            self.render_page(&page).await;
                        }
                    }
                }
            }
        }
    }

    pub(crate) async fn connect_deck(&mut self) {
        let hid_api = match new_hidapi() {
            Ok(hid) => hid,
            Err(e) => {
                print_error!("Failed to create HidApi: {}", e);
                return;
            }
        };

        loop {
            let is_connected = {
                let dev_guard = self.dev.read().await;
                match &*dev_guard {
                    Some(device) => device.keep_alive().await.is_ok(),
                    None => false,
                }
            };

            if is_connected {
                sleep(Duration::from_secs(5)).await;
                continue;
            } else {
                *self.dev.write().await = None;
            }

            print_debug!("Searching for AJazz devices...");
            let mut attempt_count = 0;
            let mut connected = false;

            while attempt_count < 5 && !connected {
                for (kind, serial) in list_devices(&hid_api) {
                    print_debug!("Found device: {:?} {}", kind, serial);

                    match ajazz_sdk::AsyncAjazz::connect(&hid_api, kind, &serial) {
                        Ok(device) => {
                            *self.dev.write().await = Some(device.clone());

                            let reader = device.get_reader();
                            tokio::spawn(self.clone().listen_device_event(reader));

                            connected = true;
                            let (page, _profile) = self.get_active_page().await.unwrap();
                            self.render_page(&page).await;
                            break;
                        }
                        Err(e) => {
                            print_error!("Failed to connect: {}", e);
                        }
                    }
                }

                if !connected {
                    attempt_count += 1;
                    print_debug!("Attempt {}/5. Retrying in 2 seconds...", attempt_count);
                    sleep(Duration::from_secs(2)).await;
                }
            }

            if !connected {
                print_warning!("Failed to connect after 5 attempts. Retrying in 15 seconds...");
                sleep(Duration::from_secs(15)).await;
            }
        }
    }

    async fn get_active_page(&self) -> Option<(Page, Profile)> {
        let data_guard = self.data.read().await;
        let Some(profile) = data_guard.profiles.get(&data_guard.active_app) else {
            print_warning!("no profile for active app");
            return None;
        };

        let Some(page) = profile.pages.get(&data_guard.active_page) else {
            print_warning!("no page for active page");
            return None;
        };

        Some((page.clone(), profile.clone()))
    }

    async fn listen_device_event(self, dev_reader: Arc<AsyncDeviceStateReader>) {
        loop {
            match dev_reader.read(100.0).await {
                Ok(updates) => {
                    let Ok(mut performer) = Performer::new() else {
                        print_error!("failed to create performer");
                        continue;
                    };

                    for update in updates {
                        match update {
                            DeviceStateUpdate::ButtonDown(key) => {
                                let (_profile, page) = {
                                    let data_guard = self.data.read().await;
                                    let Some(profile) =
                                        data_guard.profiles.get(&data_guard.active_app)
                                    else {
                                        print_warning!("no profile for active app");
                                        continue;
                                    };
                                    let Some(page) = profile.pages.get(&data_guard.active_page)
                                    else {
                                        print_warning!("no page for active page");
                                        continue;
                                    };
                                    print_debug!("active_page: {}", data_guard.active_page);
                                    (profile.clone(), page.clone())
                                };

                                let Some(button) = page.buttons.get(key as usize) else {
                                    print_warning!("no button for key: {}", key);
                                    continue;
                                };

                                let Some(button) = button else {
                                    print_warning!("no button for key: {}", key);
                                    continue;
                                };

                                match &button.action {
                                    Action::Keys(keys) => {
                                        for combo in keys {
                                            if let Err(e) = performer.press(combo) {
                                                print_error!("error pressing key: {:?}", e);
                                            }
                                        }
                                    }
                                    Action::Command(command, args) => {
                                        let output = run_command(command, args).await;
                                        print_info!("command: {:?}", output);
                                    }
                                    Action::Navigate(target_page_name) => {
                                        let mut data_guard = self.data.write().await;
                                        data_guard.active_page = target_page_name.clone();
                                        let Some(profile) =
                                            data_guard.profiles.get(&data_guard.active_app)
                                        else {
                                            print_warning!("no profile for active app");
                                            continue;
                                        };
                                        print_debug!("navigate to: {}", target_page_name);
                                        print_debug!("profile: {:?}", profile);
                                        let Some(page) = profile.pages.get(target_page_name) else {
                                            print_warning!(
                                                "no page for target page: {}",
                                                target_page_name
                                            );
                                            continue;
                                        };
                                        self.render_page(page).await;
                                    }
                                }
                            }
                            DeviceStateUpdate::ButtonUp(key) => {
                                let (_profile, page) = {
                                    let data_guard = self.data.read().await;
                                    let Some(profile) =
                                        data_guard.profiles.get(&data_guard.active_app)
                                    else {
                                        print_warning!("no profile for active app");
                                        continue;
                                    };
                                    let Some(page) = profile.pages.get(&data_guard.active_page)
                                    else {
                                        print_warning!("no page for active page");
                                        continue;
                                    };
                                    (profile.clone(), page.clone())
                                };

                                let Some(button) = page.buttons.get(key as usize) else {
                                    print_warning!("no button for key: {}", key);
                                    continue;
                                };

                                let Some(button) = button else {
                                    print_warning!("no button for key: {}", key);
                                    continue;
                                };
                                if let Action::Keys(keys) = &button.action {
                                    for combo in keys {
                                        if let Err(e) = performer.release(combo) {
                                            print_error!("error releasing key: {:?}", e);
                                        }
                                    }
                                }
                            }
                            DeviceStateUpdate::EncoderTwist(dial, ticks) => {
                                let encoder_action = {
                                    let data_guard = self.data.read().await;
                                    let Some(profile) =
                                        data_guard.profiles.get(&data_guard.active_app)
                                    else {
                                        print_warning!("no profile for active app");
                                        continue;
                                    };
                                    match profile.encoders.get(dial as usize) {
                                        Some(actions) => actions.clone(),
                                        None => continue,
                                    }
                                };

                                if let Some(encoder_action) = encoder_action {
                                    let action = if ticks > 0 {
                                        encoder_action.increment
                                    } else {
                                        encoder_action.decrement
                                    };

                                    if action.keys[0] == DECK_BRIGHTNESS_UP
                                        || action.keys[0] == DECK_BRIGHTNESS_DOWN
                                    {
                                        if ticks > 0 {
                                            self.brightness.fetch_add(5, Ordering::Relaxed);
                                        } else {
                                            self.brightness.fetch_sub(5, Ordering::Relaxed);
                                        }
                                        self.update_brightness().await;
                                        continue;
                                    }

                                    if let Err(e) = performer.perform(&action) {
                                        print_error!("error performing action: {:?}", e);
                                    }
                                }
                            }
                            DeviceStateUpdate::EncoderDown(dial) => {
                                print_debug!("encoder {} pressed", dial);
                            }
                            DeviceStateUpdate::EncoderUp(dial) => {
                                print_debug!("encoder {} released", dial);
                            }
                        }
                    }
                }
                Err(e) => {
                    print_error!("error reading device events: {}", e);
                    break;
                }
            }
        }
    }
}
