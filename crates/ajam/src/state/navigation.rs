use thiserror::Error;

use crate::print_debug;
use colored::Colorize;

use super::{
    render::{RenderError, StateRender},
    State,
};

#[derive(Error, Debug)]
pub enum NavigationError {
    #[error("no profile")]
    NoProfile,

    #[error("no page")]
    NoPage,

    #[error("render error")]
    RenderError(#[from] RenderError),
}

const DEFAULT_PROFILE: &str = "common";
const DEFAULT_PAGE: &str = "main";

pub trait StateNavigator {
    async fn navigate_to(&self, profile: &str, page: &str) -> Result<(), NavigationError>;
    async fn navigate_to_page(&self, page: &str) -> Result<(), NavigationError>;
    async fn navigate_to_default(&self) -> Result<(), NavigationError>;
    async fn navigate_to_profile_or_default(&self, profile: &str) -> Result<(), NavigationError>;
}

impl StateNavigator for State {
    async fn navigate_to(
        &self,
        profile_name: &str,
        page_name: &str,
    ) -> Result<(), NavigationError> {
        print_debug!("Navigating to {}::{}", profile_name, page_name);
        let profile = {
            let profiles_guard = self.profiles.read().await;
            if let Some(profile) = profiles_guard.get(profile_name) {
                profile.clone()
            } else {
                return Err(NavigationError::NoProfile);
            }
        };

        let Some(page) = profile.pages.get(page_name) else {
            return Err(NavigationError::NoPage);
        };

        {
            let mut navigation_guard = self.navigation.write().await;
            navigation_guard.profile = profile_name.to_string();
            navigation_guard.page = page_name.to_string();
        }

        self.render_page(page).await?;
        Ok(())
    }

    async fn navigate_to_default(&self) -> Result<(), NavigationError> {
        self.navigate_to(DEFAULT_PROFILE, DEFAULT_PAGE).await
    }

    async fn navigate_to_page(&self, page: &str) -> Result<(), NavigationError> {
        let profile = {
            let navigation = self.navigation.read().await;
            navigation.profile.clone()
        };

        self.navigate_to(&profile, page).await
    }

    async fn navigate_to_profile_or_default(&self, profile: &str) -> Result<(), NavigationError> {
        match self.navigate_to(profile, DEFAULT_PAGE).await {
            Ok(_) => Ok(()),
            Err(NavigationError::NoProfile) => self.navigate_to_default().await,
            Err(e) => Err(e),
        }
    }
}
