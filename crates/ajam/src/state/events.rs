use std::sync::Arc;

use ajam_keypress::Performer;
use ajam_profile::{Action, EncoderActions};
use ajazz_sdk::asynchronous::AsyncDeviceStateReader;
use ajazz_sdk::DeviceStateUpdate;
use tokio::process::Command;

use crate::state::render::StateRender;
use crate::state::State;
use crate::{print_debug, print_error, print_warning};
use colored::Colorize;

use super::navigation::{NavigationError, Navigator};

const KEY_PREVIOUS: u8 = 6;
const KEY_HOME: u8 = 7;
const KEY_NEXT: u8 = 8;

impl State {
    async fn handle_navigation_buttons(&self, key: u8) -> Result<Option<()>, NavigationError> {
        match key {
            KEY_PREVIOUS => self.navigate_to_previous_page().await?,
            KEY_NEXT => self.navigate_to_next_page().await?,
            KEY_HOME => self.toggle_home().await?,
            _ => return Ok(None),
        };
        Ok(Some(()))
    }

    async fn get_button_action(&self, key: u8) -> Option<Action> {
        let Some((_profile, page)) = self.get_active_page().await else {
            print_warning!("no active page found");
            return None;
        };

        let Some(button) = page.get_button(key) else {
            print_warning!("no button for key: {}", key);
            return None;
        };

        Some(button.action.clone())
    }

    async fn get_encoder_actions(&self, dial: u8) -> Option<EncoderActions> {
        let Some((profile, _page)) = self.get_active_page().await else {
            print_warning!("no active profile found");
            return None;
        };

        match profile.manifest.get_encoder_actions(dial) {
            Some(actions) => Some(actions.clone()),
            None => {
                print_warning!("no encoder action found");
                None
            }
        }
    }

    async fn execute_action(&self, action: Action, performer: &mut Performer, release: bool) {
        match action {
            Action::Keys { keys } => {
                if let Err(e) = {
                    if release {
                        performer.perform(&keys)
                    } else {
                        performer.press(&keys)
                    }
                } {
                    print_error!("error pressing key: {:?}", e);
                }
            }
            Action::Command { command } => {
                if let Err(e) = run_command(&command).await {
                    print_error!("error running command: {:?}", e);
                }
            }
            Action::Navigate { navigate } => {
                if let Err(e) = self.navigate_to_page(&navigate).await {
                    print_error!("error navigating to page: {:?}", e);
                }
            }
        }
    }
}
pub trait StateEventsHandler {
    async fn listen_device_events(&self, dev_reader: Arc<AsyncDeviceStateReader>);
}

impl StateEventsHandler for State {
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
                                match self.handle_navigation_buttons(key).await {
                                    Ok(None) => {}
                                    Ok(Some(_)) => {
                                        continue;
                                    }
                                    Err(e) => {
                                        print_error!("error navigating: {:?}", e);
                                        continue;
                                    }
                                }

                                let Some(action) = self.get_button_action(key).await else {
                                    continue;
                                };

                                self.execute_action(action, &mut performer, false).await;
                            }
                            DeviceStateUpdate::ButtonUp(key) => {
                                let Some(action) = self.get_button_action(key).await else {
                                    continue;
                                };

                                if let Action::Keys { keys } = &action {
                                    if let Err(e) = performer.release(keys) {
                                        print_error!("error releasing key: {:?}", e);
                                    }
                                }
                            }
                            DeviceStateUpdate::EncoderTwist(dial, ticks) => {
                                let Some(encoder_actions) = self.get_encoder_actions(dial).await
                                else {
                                    continue;
                                };

                                let action = if ticks > 0 {
                                    encoder_actions.plus
                                } else {
                                    encoder_actions.minus
                                };

                                if let Action::Keys { keys } = &action {
                                    if keys.is_illumination() {
                                        if let Err(e) = self.set_brightness(ticks * 5).await {
                                            print_error!("error setting brightness: {:?}", e);
                                        }
                                        continue;
                                    }
                                }

                                self.execute_action(action, &mut performer, true).await;
                            }
                            DeviceStateUpdate::EncoderDown(dial) => {
                                let Some(encoder_actions) = self.get_encoder_actions(dial).await else {
                                    continue;
                                };

                                let Some(action) = encoder_actions.click else {
                                    print_warning!("no click action found");
                                    continue;
                                };

                                self.execute_action(action, &mut performer, true).await;
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

pub(crate) async fn run_command(command: &str) -> Result<String, String> {
    print_debug!("running command: {:?}", command);
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .expect("failed to run command");
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
