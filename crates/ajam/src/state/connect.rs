use crate::{print_debug, print_error, print_warning};
use crate::state::State;
use ajazz_sdk::{list_devices, new_hidapi};
use std::time::Duration;
use tokio::time::sleep;
use colored::Colorize;

use crate::state::render::StateRender;
use crate::state::events::StateEventsHandler;

const MAX_CONNECTION_ATTEMPTS: u8 = 10;
const CONNECTION_RETRY_INTERVAL: u64 = 1;
const CONNECTION_FAILURE_RETRY_INTERVAL: u64 = 10;
const DEVICE_KEEPALIVE_CHECK_INTERVAL: u64 = 5;

pub trait StateConnect {
    async fn connect_deck(&mut self);
}

impl StateConnect for State {
    async fn connect_deck(&mut self) {
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
                sleep(Duration::from_secs(DEVICE_KEEPALIVE_CHECK_INTERVAL)).await;
                continue;
            } else {
                *self.dev.write().await = None;
            }

            print_debug!("Searching for AJazz devices...");
            let mut attempt_count = 0;
            let mut connected = false;

            while attempt_count < MAX_CONNECTION_ATTEMPTS && !connected {
                for (kind, serial) in list_devices(&hid_api) {
                    print_debug!("found device: {:?} {}", kind, serial);

                    match ajazz_sdk::AsyncAjazz::connect(&hid_api, kind, &serial) {
                        Ok(device) => {
                            *self.dev.write().await = Some(device.clone());

                            let reader = device.get_reader();
                            let state_clone = self.clone();
                            tokio::spawn(async move {
                                state_clone.listen_device_events(reader).await;
                            });

                            connected = true;
                            if let Err(e) = self.apply_brightness().await {
                                print_error!("failed to apply brightness: {}", e);
                            }
                            if let Err(e) = self.render_active_page().await {
                                print_error!("failed to render active page: {}", e);
                            }
                            break;
                        }
                        Err(e) => {
                            print_error!("failed to connect: {}", e);
                        }
                    }
                }

                if !connected {
                    attempt_count += 1;
                    print_debug!("attempt {}/{MAX_CONNECTION_ATTEMPTS}. retrying in {CONNECTION_RETRY_INTERVAL} seconds...", attempt_count);
                    sleep(Duration::from_secs(CONNECTION_RETRY_INTERVAL)).await;
                }
            }

            if !connected {
                print_warning!("failed to connect after {MAX_CONNECTION_ATTEMPTS} attempts. retrying in {CONNECTION_FAILURE_RETRY_INTERVAL} seconds...");
                sleep(Duration::from_secs(CONNECTION_FAILURE_RETRY_INTERVAL)).await;
            }
        }
    }
}