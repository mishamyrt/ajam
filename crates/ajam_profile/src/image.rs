use image::{open, DynamicImage};
use lru::LruCache;
use std::{collections::HashMap, num::NonZeroUsize, path::{Path, PathBuf}};
use thiserror::Error;

use serde::Deserialize;

/// ButtonImage is an image for a screen button.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ButtonImage {
    /// Source is a path to an static image file.
    Source { src: String },
    /// AudioInput is a map of audio input device name to a path to an image file.
    /// Must include "default" key.
    AudioInput {
        audio_input: HashMap<String, String>,
    },
    /// AudioOutput is a map of audio output device name to a path to an image file.
    /// Must include "default" key.
    AudioOutput {
        audio_output: HashMap<String, String>,
    },
}

/// ImageLoader is a trait for loading images.
pub trait ImageLoader {
    fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<DynamicImage, ImageError>;
    fn open_from_image_map(
        &mut self,
        images: &HashMap<String, String>,
        key: &str,
    ) -> Result<DynamicImage, ImageError>;
}

/// ImageCache is a cache for images.
#[derive(Debug, Clone)]
pub struct ImageCache(LruCache<String, DynamicImage>);

impl ImageCache {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self(LruCache::new(capacity))
    }

    pub fn get(&mut self, key: &str) -> Option<&DynamicImage> {
        self.0.get(key)
    }

    pub fn put(&mut self, key: String, image: DynamicImage) {
        self.0.put(key, image);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// ButtonImageLoader is a loader for button images.
#[derive(Debug)]
pub struct ButtonImageLoader<'a> {
    cache: &'a mut ImageCache,
    profile_path: PathBuf,
}

/// ImageError is an error for loading images.
#[derive(Error, Debug)]
pub enum ImageError {
    #[error("image error: {0}")]
    LoadError(String, image::ImageError),

    #[error("image not found: {0}")]
    ImageNotFound(String),

    #[error("default image not found at {0}")]
    DefaultImageNotFound(String),
}

/// ButtonImageLoader is a loader for button images.
impl<'a> ButtonImageLoader<'a> {
    pub fn new(cache: &'a mut ImageCache, profile_path: PathBuf) -> Self {
        Self {
            cache,
            profile_path,
        }
    }
}

const DEFAULT_IMAGE: &str = "default";

/// ImageLoader is a trait for loading images.
impl ImageLoader for ButtonImageLoader<'_> {
    /// Open an image from a path.
    fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<DynamicImage, ImageError> {
        let image_path = path.as_ref();
        let file_path = self.profile_path.join(image_path);
        let cache_key_buf = file_path.to_string_lossy();
        let cache_key = cache_key_buf.as_ref();
        if let Some(image) = self.cache.get(cache_key) {
            return Ok(image.clone());
        };

        match open(&file_path) {
            Ok(image) => {
                self.cache.put(cache_key.to_string(), image.clone());
                Ok(image.clone())
            }
            Err(e) => Err(ImageError::LoadError(cache_key.to_string(), e)),
        }
    }

    /// Open an image from a map of images.
    ///
    /// If the key is not found, the default image is used.
    fn open_from_image_map(
        &mut self,
        images: &HashMap<String, String>,
        key: &str,
    ) -> Result<DynamicImage, ImageError> {
        let Some(image) = images.get(key) else {
            let default = images
                .get(DEFAULT_IMAGE)
                .ok_or(ImageError::DefaultImageNotFound(key.to_string()))?;
            return self.open(default);
        };

        self.open(image)
    }
}
