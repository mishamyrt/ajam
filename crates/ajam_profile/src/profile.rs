use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::image::{ButtonImageLoader, ImageCache};
use crate::manifest::Manifest;
use crate::ProfileError;

const MANIFEST_FILE_NAME: &str = "manifest.yaml";

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub manifest: Manifest,
    path: PathBuf,
}

impl Profile {
    pub fn from_dir(path: PathBuf) -> Result<Self, ProfileError> {
        let manifest = Manifest::from_file(path.join(MANIFEST_FILE_NAME))?;
        let name = path.file_name().unwrap().to_str().unwrap().to_string();

        Ok(Self { name, manifest, path })
    }

    pub fn get_loader<'a>(&'a self, cache: &'a mut ImageCache) -> ButtonImageLoader<'a> {
        ButtonImageLoader::new(cache, self.path.clone())
    }
}

pub fn open_profiles<P: AsRef<Path>>(dir: P) -> Result<HashMap<String, Profile>, ProfileError> {
    let dir = dir.as_ref();
    let mut profiles = HashMap::new();
    for entry in dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let profile = Profile::from_dir(path)?;
            profiles.insert(profile.name.clone(), profile);
        }
    }
    Ok(profiles)
}
