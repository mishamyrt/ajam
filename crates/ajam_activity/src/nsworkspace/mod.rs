mod app_delegate;
mod app_state;
mod util;
mod listener;

use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NSWorkspaceError {
    #[error("Failed to get frontmost application")]
    GetFrontmostApplication,
    #[error("Failed to get bundle identifier")]
    GetBundleIdentifier,
    #[error("Failed to get UTF8 string")]
    GetUTF8String,
    #[error("Failed to convert string")]
    ConvertStringError(Utf8Error),
    #[error("Failed to send event")]
    SendEventError(std::sync::mpsc::SendError<Event>),
    #[error("Failed to get user info")]
    GetUserInfo,
}

pub(crate) use listener::start_nsworkspace_listener;

use crate::Event;