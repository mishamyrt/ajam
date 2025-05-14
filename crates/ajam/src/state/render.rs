use std::sync::atomic::Ordering;

use ajam_profile::{Page, Profile};
use ajazz_sdk::AjazzError;
use image::{open, DynamicImage};
use thiserror::Error;

use crate::State;

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("no device")]
    NoDevice,

    #[error("button index out of bounds: {0}")]
    ButtonIndexOutOfBounds(usize),

    #[error("error writing to device")]
    DeviceWriteError(#[from] AjazzError),

    #[error("error loading image: {0}")]
    ImageError(#[from] image::ImageError),

    #[error("no active page")]
    NoActivePage,
}

struct RenderState(Vec<Option<DynamicImage>>);

impl RenderState {
    pub fn from_page(page: &Page) -> Result<Self, RenderError> {
        let mut images = Vec::new();
        for button in page.buttons.iter() {
            if let Some(button) = button {
                match open(button.image.clone()) {
                    Ok(image) => images.push(Some(image)),
                    Err(e) => {
                        return Err(RenderError::ImageError(e));
                    }
                }
            } else {
                images.push(None);
            }
        }
        Ok(Self(images))
    }
}

impl State {
    async fn render_state(&self, state: &RenderState) -> Result<(), RenderError> {
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
}

pub trait StateRender {
    async fn render_page(&self, page: &Page) -> Result<(), RenderError>;
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

    async fn render_page(&self, page: &Page) -> Result<(), RenderError> {
        let state = RenderState::from_page(page)?;
        self.render_state(&state).await
    }

    async fn render_active_page(&self) -> Result<(), RenderError> {
        let Some((_, page)) = self.get_active_page().await else {
            return Err(RenderError::NoActivePage);
        };
        self.render_page(&page).await
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
