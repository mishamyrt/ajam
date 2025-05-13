use std::{rc::Rc, sync::Mutex};

use enigo::{Enigo, InputResult, NewConError, Settings};

use crate::KeyCombo;

pub struct Performer {
    enigo: Rc<Mutex<Enigo>>,
}

// SAFETY: This is safe because we're only accessing Enigo through a Mutex,
// which provides the necessary synchronization. The internal CGEventSource
// is only used on the thread that actually performs the key presses.
unsafe impl Send for Performer {}
unsafe impl Sync for Performer {}

impl Performer {
    pub fn new() -> Result<Self, NewConError> {
        let settings = Settings::default();
        let enigo = Enigo::new(&settings)?;
        Ok(Self {
            enigo: Rc::new(Mutex::new(enigo)),
        })
    }

    pub fn perform(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        let mut enigo = self.enigo.lock().unwrap();
        key_combo.perform(&mut enigo)
    }

    pub fn press(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        let mut enigo = self.enigo.lock().unwrap();
        key_combo.press(&mut enigo)
    }

    pub fn release(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        let mut enigo = self.enigo.lock().unwrap();
        key_combo.release(&mut enigo)
    }
}
