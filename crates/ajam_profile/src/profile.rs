
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::manifest::{Action as ManifestAction, Manifest, Page as ManifestPage};
use ajam_keypress::KeyCombo;

#[derive(Debug, Clone)]
pub struct EncoderActions {
    pub increment: KeyCombo,
    pub decrement: KeyCombo,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub app_id: String,
    pub path: PathBuf,
    pub pages: HashMap<String, Page>,
    pub encoders: Vec<Option<EncoderActions>>,
    pub buttons_per_page: usize,
}

#[derive(Debug, Clone)]
pub enum Action {
    Keys(Vec<KeyCombo>),
    Command(String, Vec<String>),
    Navigate(String),
}

#[derive(Debug, Clone)]
pub struct Button {
    pub image: PathBuf,
    pub action: Action,
}

#[derive(Debug)]
pub struct Page {
    pub buttons: Vec<Option<Button>>,
}

impl Action {
    pub fn from_manifest(action: ManifestAction) -> Option<Self> {
        match action {
            ManifestAction::Keys { keys } => {
                let mut combos = Vec::new();
                for k in keys {
                    let combo = KeyCombo::from_str(&k);
                    if let Ok(combo) = combo {
                        combos.push(combo);
                    } else {
                        return None;
                    }
                }
                Some(Action::Keys(combos))
            }
            ManifestAction::Command { command } => {
                let args = command.split_whitespace().collect::<Vec<&str>>();
                let command = args[0];
                let args = args[1..].to_vec();
                Some(Action::Command(
                    command.to_string(),
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            }
            ManifestAction::Navigate { navigate } => Some(Action::Navigate(navigate.to_string())),
        }
    }
}

impl Profile {
    pub fn from_manifest(
        manifest: Manifest,
        path: PathBuf,
        buttons_per_page: usize,
    ) -> Result<Self, crate::ProfileError> {
        let mut pages = HashMap::new();

        for (page_name, manifest_page) in manifest.pages {
            let Some(page) =
                Page::from_manifest_page(manifest_page, buttons_per_page, path.clone())
            else {
                return Err(crate::ProfileError::InvalidManifest);
            };
            pages.insert(page_name, page);
        }

        let mut encoders = vec![None; manifest.encoders.len()];

        for (key_char, encoder) in manifest.encoders {
            if let Some(index) = key_char.to_digit(10) {
                let index = index as usize;
                if encoder.len() == 2 {
                    let Ok(decrement) = KeyCombo::from_str(&encoder[0]) else {
                        return Err(crate::ProfileError::InvalidManifest);
                    };
                    let Ok(increment) = KeyCombo::from_str(&encoder[1]) else {
                        return Err(crate::ProfileError::InvalidManifest);
                    };
                    encoders[index] = Some(EncoderActions {
                        increment,
                        decrement,
                    });
                }
            }
        }
        Ok(Profile {
            app_id: manifest.app_id,
            path,
            pages,
            encoders,
            buttons_per_page,
        })
    }

    pub fn from_file<P: AsRef<Path>>(
        path: P,
        buttons_per_page: usize,
    ) -> Result<Self, crate::ProfileError> {
        let path_buf = path.as_ref().to_path_buf();
        let manifest = Manifest::from_file(&path_buf)?;
        Self::from_manifest(
            manifest,
            path_buf.parent().unwrap_or(Path::new("")).to_path_buf(),
            buttons_per_page,
        )
    }
}

impl Page {
    fn from_manifest_page(
        manifest_page: ManifestPage,
        buttons_per_page: usize,
        images_dir: PathBuf,
    ) -> Option<Self> {
        let mut buttons: Vec<Option<Button>> = vec![None; buttons_per_page];

        for (key_char, button) in manifest_page.buttons {
            if let Some(index) = key_char.to_digit(10) {
                let index = index as usize;
                if index < buttons_per_page {
                    let action = Action::from_manifest(button.action)?;
                    buttons[index] = Some(Button {
                        image: images_dir.join(button.image),
                        action,
                    });
                }
            }
        }

        Some(Page { buttons })
    }
}

impl Clone for Page {
    fn clone(&self) -> Self {
        Page {
            buttons: self.buttons.clone(),
        }
    }
}
