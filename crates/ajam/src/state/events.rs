use std::sync::{mpsc, Arc};

use ajam_events::ActivityEvent;
use ajam_keypress::{Performer, DECK_BRIGHTNESS_DOWN, DECK_BRIGHTNESS_UP};
use ajam_profile::Action;
use ajazz_sdk::asynchronous::AsyncDeviceStateReader;
use ajazz_sdk::DeviceStateUpdate;
use tokio::process::Command;

use crate::state::render::StateRender;
use crate::state::State;
use crate::{print_debug, print_error, print_warning};
use colored::Colorize;

use super::navigation::StateNavigator;
use super::navigation::DEFAULT_PROFILE;

const KEY_PREVIOUS: u8 = 6;
const KEY_HOME: u8 = 7;
const KEY_NEXT: u8 = 8;

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
                        let profile = {
                            let navigation_guard = self.navigation.read().await;
                            navigation_guard.profile.clone()
                        };

                        if profile == bundle_id {
                            continue;
                        }
                        last_bundle_id = Some(bundle_id.clone());

                        {
                            let mut active_profile_guard = self.active_profile.write().await;
                            *active_profile_guard = DEFAULT_PROFILE.to_string();
                        }

                        if let Err(e) = self.navigate_to_profile_or_default(&bundle_id).await {
                            print_error!("error navigating to profile: {:?}", e);
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
                                match key {
                                    KEY_PREVIOUS => {
                                        match self.navigate_to_previous_page().await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                print_error!("error navigating to previous page: {:?}", e);
                                            }
                                        }
                                        continue;
                                    }
                                    KEY_NEXT => {
                                        match self.navigate_to_next_page().await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                print_error!("error navigating to next page: {:?}", e);
                                            }
                                        }
                                        continue;
                                    }
                                    KEY_HOME => {
                                        match self.toggle_home().await {
                                            Ok(_) => {}
                                            Err(e) => {
                                                print_error!("error toggling home: {:?}", e);
                                            }
                                        }
                                        continue;
                                    }
                                    _ => {}
                                };

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
                                        if let Err(e) =
                                            self.navigate_to_page(target_page_name).await
                                        {
                                            print_error!("error navigating to page: {:?}", e);
                                        }
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
                                        if let Err(e) = self.set_brightness(ticks * 5).await {
                                            print_error!("error setting brightness: {:?}", e);
                                        }
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
