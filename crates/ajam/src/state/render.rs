use std::sync::atomic::Ordering;

use image::DynamicImage;
use thiserror::Error;

use ajam_profile::{ButtonImage, ImageLoader, Page, Profile};
use ajazz_sdk::AjazzError;

use crate::State;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("no device")]
    NoDevice,

    #[error("button index out of bounds: {0}")]
    ButtonIndexOutOfBounds(usize),

    #[error("error writing to device")]
    DeviceWriteError(#[from] AjazzError),

    #[error("error loading image")]
    ImageError(#[from] image::ImageError),

    #[error("no active page")]
    NoActivePage,

    #[error("error loading page")]
    PageLoadError(#[from] ajam_profile::ProfileError),

    #[error("error loading image from source: {0}")]
    ImageSourceError(#[from] ajam_profile::ImageError),
}

struct MaterializedPage(Vec<Option<DynamicImage>>);


impl State {
    async fn render_state(&self, state: &MaterializedPage) -> Result<(), RenderError> {
        // Maybe too short lock?
        let dev = {
            let dev_guard = self.dev.read().await;
            match &*dev_guard {
                Some(dev) => dev.clone(),
                None => return Err(RenderError::NoDevice),
            }
        };

        dev.clear_all_button_images().await?;
        for (i, image) in state.0.iter().enumerate() {
            if i >= dev.kind().key_count() as usize {
                return Err(RenderError::ButtonIndexOutOfBounds(i));
            }

            if let Some(image) = image {
                dev.set_button_image(i as u8, image.clone()).await?;
            }
        }
        dev.flush().await?;

        Ok(())
    }

    async fn materialize_page(&self, profile: &Profile, page: &Page) -> Result<MaterializedPage, RenderError> {
        let buttons_count = profile.manifest.kind().display_key_count() as usize;

        let mut image_cache = self.image_cache.lock().await;
        let mut loader = profile.get_loader(&mut image_cache);
        let mut images: Vec<Option<DynamicImage>> = vec![None; buttons_count];
        
        for (i, button) in page.iter_buttons(buttons_count).enumerate() {
            let Some(button) = button else {
                return Err(RenderError::ButtonIndexOutOfBounds(i));
            };

            let image = match &button.image {
                ButtonImage::Source { src } => {
                    loader.open(src)?
                }
                ButtonImage::AudioInput { audio_input } => {
                    let input_device_name = self.audio_input_device.read().await;
                    loader.open_from_image_map(audio_input, &input_device_name)?
                }
                ButtonImage::AudioOutput { audio_output } => {
                    let output_device_name = self.audio_output_device.read().await;
                    loader.open_from_image_map(audio_output, &output_device_name)?
                }
            };
            images[i] = Some(image)
        }

        Ok(MaterializedPage(images))
    }
}

pub trait StateRender {
    async fn render_page(&self, profile: &Profile, page: &Page) -> Result<(), RenderError>;
    async fn render_active_page(&self) -> Result<(), RenderError>;

    async fn apply_brightness(&self) -> Result<(), RenderError>;
    async fn set_brightness(&self, delta: i8) -> Result<(), RenderError>;

    async fn get_active_page(&self) -> Option<(Profile, Page)>;
}

impl StateRender for State {
    async fn get_active_page(&self) -> Option<(Profile, Page)> {
        let (profile, page) = {
            let navigation_guard = self.navigation.read().await;

            (
                navigation_guard.profile.clone(),
                navigation_guard.page.clone(),
            )
        };

        self.get_page(&profile, &page).await
    }

    async fn render_page(&self, profile: &Profile, page: &Page) -> Result<(), RenderError> {
        let materialized_page = self.materialize_page(profile, page).await?;
        self.render_state(&materialized_page).await
    }

    async fn render_active_page(&self) -> Result<(), RenderError> {
        let Some((profile, page)) = self.get_active_page().await else {
            return Err(RenderError::NoActivePage);
        };
        self.render_page(&profile, &page).await
    }

    async fn apply_brightness(&self) -> Result<(), RenderError> {
        let brightness = self.brightness.load(Ordering::Relaxed);
        let dev_guard = self.dev.read().await;
        let Some(dev) = dev_guard.as_ref() else {
            return Err(RenderError::NoDevice);
        };

        dev.set_brightness(brightness).await?;
        Ok(())
    }

    async fn set_brightness(&self, delta: i8) -> Result<(), RenderError> {
        let abs_delta = delta.unsigned_abs();
        if delta > 0 {
            self.brightness.fetch_add(abs_delta, Ordering::Relaxed);
        } else {
            self.brightness.fetch_sub(abs_delta, Ordering::Relaxed);
        }
        self.apply_brightness().await
    }
}
