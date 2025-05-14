use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::manifest::{Action as ManifestAction, Manifest, Page as ManifestPage};
use crate::ProfileError;
use ajam_keypress::KeyCombo;

const MANIFEST_FILE_NAME: &str = "manifest.yaml";

#[derive(Debug, Clone)]
pub struct EncoderActions {
    pub plus: Action,
    pub minus: Action,
    pub click: Option<Action>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub app_id: String,
    pub path: PathBuf,
    pub pages: HashMap<String, Page>,
    pub pages_order: Vec<String>,
    pub encoders: Vec<Option<EncoderActions>>,
    pub buttons_per_page: usize,
}

#[derive(Debug, Clone)]
pub enum Action {
    Keys(KeyCombo),
    Command(String),
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
    pub fn from_manifest(action: ManifestAction) -> Result<Self, crate::ProfileError> {
        match action {
            ManifestAction::Keys { keys } => {
                let Ok(combo) = KeyCombo::from_str(&keys) else {
                    return Err(crate::ProfileError::InvalidKeyCombo(keys));
                };
                Ok(Action::Keys(combo))
            }
            ManifestAction::Command { command } => Ok(Action::Command(command.to_string())),
            ManifestAction::Navigate { navigate } => Ok(Action::Navigate(navigate.to_string())),
        }
    }
}

impl Profile {
    pub fn from_manifest(
        app_id: String,
        manifest: Manifest,
        images_dir: PathBuf,
        buttons_per_page: usize,
    ) -> Result<Self, crate::ProfileError> {
        let mut pages = HashMap::new();

        for (page_name, manifest_page) in manifest.pages {
            let page =
                Page::from_manifest_page(manifest_page, buttons_per_page, images_dir.clone())?;
            pages.insert(page_name, page);
        }

        let mut encoders = vec![None; manifest.encoders.len()];

        for (key_char, encoder) in manifest.encoders {
            if let Some(index) = key_char.to_digit(10) {
                let index = index as usize;
                let click = match encoder.click {
                    Some(action) => Some(Action::from_manifest(action)?),
                    None => None,
                };
                encoders[index] = Some(EncoderActions {
                    plus: Action::from_manifest(encoder.plus)?,
                    minus: Action::from_manifest(encoder.minus)?,
                    click,
                });
            }
        }
        Ok(Profile {
            app_id,
            path: images_dir,
            pages,
            pages_order: manifest.pages_order,
            encoders,
            buttons_per_page,
        })
    }

    pub fn from_dir<P: AsRef<Path>>(
        path: P,
        buttons_per_page: usize,
    ) -> Result<Self, crate::ProfileError> {
        let path_buf = path.as_ref().to_path_buf();
        let manifest_path = path_buf.join(MANIFEST_FILE_NAME);

        if !manifest_path.exists() {
            return Err(ProfileError::ManifestFileNotFound(
                manifest_path.to_str().unwrap().to_string(),
            ));
        }

        let manifest = Manifest::from_file(manifest_path)?;

        let Some(app_id) = path_buf.file_name() else {
            return Err(crate::ProfileError::InvalidAppId(
                path_buf.to_str().unwrap().to_string(),
            ));
        };
        Self::from_manifest(
            app_id.to_str().unwrap().to_string(),
            manifest,
            path_buf,
            buttons_per_page,
        )
    }
}

impl Page {
    fn from_manifest_page(
        manifest_page: ManifestPage,
        buttons_per_page: usize,
        images_dir: PathBuf,
    ) -> Result<Self, crate::ProfileError> {
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

        Ok(Page { buttons })
    }
}

impl Clone for Page {
    fn clone(&self) -> Self {
        Page {
            buttons: self.buttons.clone(),
        }
    }
}

pub fn open_profiles(dir: &Path) -> Result<HashMap<String, Profile>, ProfileError> {
    let mut profiles = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let profile = Profile::from_dir(&path, 6)?;
            profiles.insert(profile.app_id.clone(), profile);
        }
    }

    Ok(profiles)
}
