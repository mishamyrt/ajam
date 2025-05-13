use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};

use ajam_events::ActivityEvent;
use ajam_keypress::{Performer, DECK_BRIGHTNESS_DOWN, DECK_BRIGHTNESS_UP};
use ajam_profile::Action;
use ajazz_sdk::asynchronous::AsyncDeviceStateReader;
use ajazz_sdk::DeviceStateUpdate;
use tokio::process::Command;

use crate::state::render::StateRender;
use crate::state::{State, DEFAULT_APP};
use crate::{print_debug, print_error, print_info, print_warning};
use colored::Colorize;

pub trait StateEventsHandler {
    async fn listen_device_events(&self, dev_reader: Arc<AsyncDeviceStateReader>);
    async fn listen_os_events(&self, rx: mpsc::Receiver<ActivityEvent>);
}

impl StateEventsHandler for State {
    async fn listen_os_events(&self, rx: mpsc::Receiver<ActivityEvent>) {
        let mut last_bundle_id: Option<String> = None;

        while let Ok(event) = rx.recv() {
            match event {
                ActivityEvent::AppChange(bundle_id) => {
                    if last_bundle_id.as_ref() != Some(&bundle_id) {
                        let page = {
                            let mut data_guard = self.data.write().await;
                            if data_guard.active_app == bundle_id {
                                continue;
                            }
                            last_bundle_id = Some(bundle_id.clone());

                            if data_guard.profiles.contains_key(&bundle_id) {
                                print_info!("updating active app: {}", bundle_id);
                                data_guard.active_app = bundle_id.to_string();
                            } else {
                                print_warning!("no profile for active app: {}", bundle_id);
                                data_guard.active_app = DEFAULT_APP.to_string();
                            }

                            let Some(profile) = data_guard.profiles.get(&data_guard.active_app)
                            else {
                                print_warning!("no profile for active app: {}", bundle_id);
                                continue;
                            };

                            let Some(page) = profile.pages.get(&data_guard.active_page) else {
                                print_warning!("no page for active page");
                                continue;
                            };

                            page.clone()
                        };

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

    async fn listen_device_events(&self, dev_reader: Arc<AsyncDeviceStateReader>) {
        let Ok(mut performer) = Performer::new() else {
            print_error!("failed to create performer");
            return;
        };

        loop {
            match dev_reader.read(100.0).await {
                Ok(updates) => {
                    for update in updates {
                        match update {
                            DeviceStateUpdate::ButtonDown(key) => {
                                let Some((_profile, page)) = self.get_active_page().await else {
                                    print_warning!("no active page found");
                                    continue;
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
                                        print_debug!("running command: {:?} {:?}", command, args);
                                        if let Err(e) = run_command(command, args).await {
                                            print_error!("error running command: {:?}", e);
                                        }
                                    }
                                    Action::Navigate(target_page_name) => {
                                        let Some((profile, _page)) = self.get_active_page().await
                                        else {
                                            print_warning!("no active page found");
                                            continue;
                                        };

                                        let Some(page) = profile.pages.get(target_page_name) else {
                                            print_warning!(
                                                "no page for target {} in profile {}",
                                                target_page_name,
                                                profile.app_id
                                            );
                                            continue;
                                        };

                                        self.data.write().await.active_page =
                                            target_page_name.clone();
                                        self.render_page(page).await;
                                    }
                                }
                            }
                            DeviceStateUpdate::ButtonUp(key) => {
                                let Some((_profile, page)) = self.get_active_page().await else {
                                    print_warning!("no active page found");
                                    continue;
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
                                    let Some((profile, _page)) = self.get_active_page().await
                                    else {
                                        print_warning!("no active profile found");
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

pub(crate) async fn run_command(command: &str, args: &Vec<String>) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await
        .expect("failed to run command");
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
