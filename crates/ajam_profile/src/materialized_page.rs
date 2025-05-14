use image::{open, DynamicImage};
use lru::LruCache;
use thiserror::Error;
use crate::profile::Page;

#[derive(Error, Debug)]
pub enum MaterializedPageError {
    #[error("error loading image at {0}: {1}")]
    ImageError(String, image::ImageError),
}

/// MaterializedPage is a page with all the images loaded.
/// If button is empty, it means that the button is not visible.
pub struct MaterializedPage(pub Vec<Option<DynamicImage>>);

impl MaterializedPage {
    pub fn new(size: usize) -> Self {
        Self(vec![None; size])
    }

    pub fn from_page(page: &Page) -> Result<Self, MaterializedPageError> {
        let mut images = Vec::with_capacity(page.buttons.len());
        for button in page.buttons.iter() {
            if let Some(button) = button {
                match open(button.image.clone()) {
                    Ok(image) => images.push(Some(image)),
                    Err(e) => {
                        return Err(MaterializedPageError::ImageError(
                            button.image.to_string_lossy().to_string(),
                            e,
                        ));
                    }
                }
            } else {
                images.push(None);
            }
        }
        Ok(Self(images))
    }
}

pub struct ImageLoader {
    cache: LruCache<String, DynamicImage>,
}

impl ImageLoader {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: LruCache::new(max_size.try_into().unwrap()),
        }
    }

    pub async fn open_page(&mut self, page: &Page) -> Result<MaterializedPage, MaterializedPageError> {
        let mut materialized_page = MaterializedPage::new(page.buttons.len());
        for (index, button) in page.buttons.iter().enumerate() {
            let Some(button) = button else {
                materialized_page.0[index] = None;
                continue;
            };
            let image = self.open(&button.image.to_string_lossy()).await?;
            materialized_page.0[index] = Some(image);
        }

        Ok(materialized_page)
    }

    pub async fn open(&mut self, path: &str) -> Result<DynamicImage, MaterializedPageError> {
        if let Some(image) = self.cache.get(path) {
            return Ok(image.clone());
        }

        println!("opening image: {}", path);

        match open(path) {
            Ok(image) => {
                self.cache.put(path.to_string(), image.clone());
                Ok(image.clone())
            }
            Err(e) => Err(MaterializedPageError::ImageError(path.to_string(), e)),
        }
    }
}
