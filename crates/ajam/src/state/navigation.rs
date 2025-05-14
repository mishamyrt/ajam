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

pub const DEFAULT_PROFILE: &str = "common";
const DEFAULT_PAGE: &str = "main";

pub trait StateNavigator {
    async fn navigate_to(&self, profile: &str, page: &str) -> Result<(), NavigationError>;
    async fn navigate_to_page(&self, page: &str) -> Result<(), NavigationError>;
    async fn navigate_to_default(&self) -> Result<(), NavigationError>;
    async fn navigate_to_profile_or_default(&self, profile: &str) -> Result<(), NavigationError>;

    async fn toggle_home(&self) -> Result<(), NavigationError>;

    async fn navigate_to_next_page(&self) -> Result<(), NavigationError>;
    async fn navigate_to_previous_page(&self) -> Result<(), NavigationError>;
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

    async fn toggle_home(&self) -> Result<(), NavigationError> {
        let profile_name = {
            let navigation_guard = self.navigation.read().await;
            navigation_guard.profile.clone()
        };

        let active_profile = {
            let active_profile_guard = self.active_profile.read().await;
            active_profile_guard.clone()
        };

        if active_profile == DEFAULT_PROFILE && profile_name == DEFAULT_PROFILE {
            print_debug!("Already on default profile and no saved profile, skipping");
            return Ok(());
        }

        if profile_name != DEFAULT_PROFILE {
            let mut active_profile_guard = self.active_profile.write().await;
            *active_profile_guard = profile_name.clone();
            return self.navigate_to_default().await;
        }

        if profile_name == DEFAULT_PROFILE && active_profile != DEFAULT_PROFILE {
            let mut active_profile_guard = self.active_profile.write().await;
            let profile_to_restore = active_profile.clone();
            *active_profile_guard = DEFAULT_PROFILE.to_string();
            return self.navigate_to_profile_or_default(&profile_to_restore).await;
        }

        Ok(())
    }

    async fn navigate_to_page(&self, page: &str) -> Result<(), NavigationError> {
        let profile = {
            let navigation = self.navigation.read().await;
            navigation.profile.clone()
        };

        self.navigate_to(&profile, page).await
    }

    async fn navigate_to_profile_or_default(&self, profile: &str) -> Result<(), NavigationError> {
        let profile_name = {
            let navigation_guard = self.navigation.read().await;
            navigation_guard.profile.clone()
        };
        if profile_name == profile {
            print_debug!("Already on profile {}, skipping", profile);
            return Ok(());
        }
        match self.navigate_to(profile, DEFAULT_PAGE).await {
            Ok(_) => Ok(()),
            Err(NavigationError::NoProfile) => {
                if profile_name == DEFAULT_PROFILE {
                    print_debug!("Already on default profile, skipping");
                    return Ok(());
                }
                self.navigate_to_default().await
            },
            Err(e) => Err(e),
        }
    }

    async fn navigate_to_next_page(&self) -> Result<(), NavigationError> {
        self.navigate_page_offset(1).await
    }
    
    async fn navigate_to_previous_page(&self) -> Result<(), NavigationError> {
        self.navigate_page_offset(-1).await
    }
}

impl State {
    async fn get_current_profile_and_page(&self) -> (String, String) {
        let navigation = self.navigation.read().await;
        (navigation.profile.clone(), navigation.page.clone())
    }

    async fn get_profile(&self, profile_name: &str) -> Result<crate::state::Profile, NavigationError> {
        let profiles_guard = self.profiles.read().await;
        profiles_guard.get(profile_name).cloned().ok_or(NavigationError::NoProfile)
    }

    async fn navigate_page_offset(&self, offset: isize) -> Result<(), NavigationError> {
        let (profile_name, page_name) = self.get_current_profile_and_page().await;
        let profile = self.get_profile(&profile_name).await?;
        let len = profile.pages_order.len() as isize;
        let current_index = profile.pages_order.iter().position(|p| p == &page_name).unwrap() as isize;
        let new_index = (current_index + offset + len) % len;
        let new_page = &profile.pages_order[new_index as usize];
        self.navigate_to(&profile_name, new_page).await
    }
}
