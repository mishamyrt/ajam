use std::sync::mpsc;
use std::thread;

use crate::coreaudio::start_coreaudio_listener;
use crate::nsworkspace::{start_nsworkspace_listener, NSWorkspaceError};

/// An event from the monitor.
#[derive(Debug, Clone)]
pub enum Event {
    AppChange(String),
    AudioOutputChange(String),
    AudioInputChange(String)
}

/// A monitor for system events.
///
/// This monitor listens for events from the core audio and workspace APIs.
/// It is designed to be used in the main thread.
pub struct Monitor {
    event_tx: mpsc::Sender<Event>,
}

impl Monitor {
    /// Create a new monitor.
    /// Returns a tuple containing the monitor and a receiver for events.
    pub fn new() -> (Self, mpsc::Receiver<Event>) {
        let (event_tx, event_rx) = mpsc::channel();
        (Self { event_tx }, event_rx)
    }

    /// Start listening for events.
    /// Must be called in the main thread.
    pub fn start_listening(self) -> Result<(), NSWorkspaceError> {
        thread::spawn({
            let tx = self.event_tx.clone();
            move || {
                if let Err(e) = start_coreaudio_listener(tx) {
                    println!("Error starting coreaudio listener: {:?}", e);
                }
            }
        });

        start_nsworkspace_listener(self.event_tx)
    }
}
