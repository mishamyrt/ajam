use std::sync::atomic::Ordering;

use ajam_profile::{Page, Profile};
use image::open;
use colored::Colorize;

use crate::{print_error, print_warning, State};

pub trait StateRender {
    async fn render_page(&self, page: &Page) -> Option<()>;
    async fn render_page_with_images(&self, images_to_render: &[(u8, image::DynamicImage)]) -> Option<()>;
    async fn update_brightness(&self);
    async fn get_active_page(&self) -> Option<(Profile, Page)>;
}

impl StateRender for State {
    async fn get_active_page(&self) -> Option<(Profile, Page)> {
        let data_guard = self.data.read().await;
        let Some(profile) = data_guard.profiles.get(&data_guard.active_app) else {
            print_warning!("no profile for active app");
            return None;
        };

        let Some(page) = profile.pages.get(&data_guard.active_page) else {
            print_warning!("no page for active page");
            return None;
        };

        Some((profile.clone(), page.clone()))
    }

    async fn render_page(&self, page: &Page) -> Option<()> {
        let images_to_render = page
            .buttons
            .iter()
            .enumerate()
            .filter_map(|(i, button)| {
                if let Some(button) = button {
                    match open(button.image.clone()) {
                        Ok(image) => Some((i as u8, image)),
                        Err(e) => {
                            print_error!(
                                "Error loading image: {} {:?}",
                                button.image.to_string_lossy(),
                                e
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        self.render_page_with_images(&images_to_render).await
    }

    async fn render_page_with_images(
        &self,
        images_to_render: &[(u8, image::DynamicImage)],
    ) -> Option<()> {
        let dev = {
            let dev_guard = self.dev.read().await;
            match &*dev_guard {
                Some(dev) => dev.clone(),
                None => return None,
            }
        };

        let brightness = self.brightness.load(Ordering::Relaxed);

        dev.set_brightness(brightness).await.unwrap();
        dev.clear_all_button_images().await.unwrap();

        for &(i, ref image) in images_to_render {
            dev.set_button_image(i, image.clone()).await.unwrap();
        }

        for i in 0..dev.kind().key_count() {
            if !images_to_render.iter().any(|(btn_i, _)| *btn_i == i) {
                dev.clear_button_image(i).await.unwrap();
            }
        }

        dev.flush().await.unwrap();

        Some(())
    }

    async fn update_brightness(&self) {
        let brightness = self.brightness.load(Ordering::Relaxed);
        let dev = {
            let dev_guard = self.dev.read().await;
            dev_guard.as_ref().unwrap().clone()
        };
        dev.set_brightness(brightness).await.unwrap();
    }
}