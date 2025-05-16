use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use ajazz_sdk::info::Kind;

use ajam_keypress::KeyCombo;

use crate::image::ButtonImage;

/// Action is an action that can be performed.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Action {
    /// Keys is a key combo to press.
    Keys { keys: KeyCombo },
    /// Command is a command to run in the terminal.
    Command { command: String },
    /// Navigate is a path to navigate to.
    Navigate { navigate: String },
}

/// EncoderActions is a set of actions for an encoder.
#[derive(Debug, Deserialize, Clone)]
pub struct EncoderActions {
    /// Plus is the action to perform when the encoder is turned clockwise.
    pub plus: Action,
    /// Minus is the action to perform when the encoder is turned counterclockwise.
    pub minus: Action,
    /// Click is the action to perform when the encoder is clicked.
    pub click: Option<Action>,
}

/// Button is a screen button config.
#[derive(Debug, Clone, Deserialize)]
pub struct Button {
    /// Image is the image for the button.
    pub image: ButtonImage,
    /// Action is the action to perform when the button is clicked.
    pub action: Action,
}

/// Page is a page in the manifest.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct Page {
    /// Buttons is a map of button index char to button configs.
    #[serde(flatten)]
    pub buttons: HashMap<char, Button>,
}

/// Manifest is the manifest for a profile.
#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    /// PagesOrder is the order of the pages.
    #[serde(default)]
    pub pages_order: Vec<String>,
    /// Device is the device type.
    pub device: String,
    /// Pages is a map of page names to pages.
    pub pages: HashMap<String, Page>,
    /// Encoders is a map of encoder index char to encoder actions.
    pub encoders: HashMap<char, EncoderActions>,
}

impl Manifest {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, crate::ProfileError> {
        let data = fs::read_to_string(path)?;
        let manifest: Manifest = serde_yaml::from_str(&data)?;
        Ok(manifest)
    }

    pub fn get_encoder_actions(&self, index: u8) -> Option<&EncoderActions> {
        let ch = char::from_digit(index as u32, 10)?;
        self.encoders.get(&ch)
    }

    pub fn get_page(&self, name: &str) -> Option<&Page> {
        self.pages.get(name)
    }

    pub fn page_index(&self, name: &str) -> Option<usize> {
        self.pages_order.iter().position(|page| page == name)
    }

    pub fn kind(&self) -> Kind {
        match self.device.as_str() {
            "akp03" => Kind::Akp03,
            "akp03e" => Kind::Akp03E,
            "akp03r" => Kind::Akp03R,
            "akp03r_rev2" => Kind::Akp03RRev2,
            "akp153" => Kind::Akp153,
            "akp153e" => Kind::Akp153E,
            "akp153r" => Kind::Akp153R,
            _ => panic!("Unknown device: {}", self.device),
        }
    }
}

impl Page {
    pub fn get_button(&self, index: u8) -> Option<&Button> {
        let ch = char::from_digit(index as u32, 10)?;
        self.buttons.get(&ch)
    }

    pub fn iter_buttons(&self, count: usize) -> impl Iterator<Item = Option<&Button>> {
        let mut buttons: Vec<Option<&Button>> = vec![None; count];
        for (index, button) in self.buttons.iter() {
            let index = index.to_digit(10).unwrap() as usize;
            if index >= buttons.len() {
                continue;
            }
            buttons[index] = Some(button);
        }

        buttons.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_button() {
        let mut manifest = Manifest {
            device: "akp03".to_string(),
            pages_order: vec!["test".to_string()],
            pages: HashMap::new(),
            encoders: HashMap::new(),
        };

        let mut page = Page {
            buttons: HashMap::new(),
        };

        page.buttons.insert('0', Button {
            image: ButtonImage::Source { src: "test.png".to_string() },
            action: Action::Command { command: "echo 'test'".to_string() },
        });

        manifest.pages.insert("test".to_string(), page);

        let page = manifest.get_page("test").unwrap();
        assert_eq!(page.buttons.len(), 1);
    }

    #[test]
    fn test_kind() {
        let manifest = Manifest {
            device: "akp03".to_string(),
            pages_order: vec!["test".to_string()],
            pages: HashMap::new(),
            encoders: HashMap::new(),
        };

        assert_eq!(manifest.kind(), Kind::Akp03);
    }
}
