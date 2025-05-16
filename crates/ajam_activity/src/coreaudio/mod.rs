mod property;
mod sys;
mod listener;

use std::str::Utf8Error;

use thiserror::Error;

pub(crate) use listener::start_coreaudio_listener;

use crate::Event;

#[derive(Error, Debug)]
pub enum CoreAudioError {
    #[error("Failed to read property {selector}. Status: {status}")]
    ReadProperty {
        selector: sys::AudioObjectPropertySelector,
        status: sys::OSStatus,
    },

    #[error("Failed to get read property {selector}. Size is {size}")]
    GetPropertySize {
        selector: sys::AudioObjectPropertySelector,
        size: u32,
    },

    #[error("Property is empty. Size is 0")]
    EmptyProperty,

    #[error("Invalid buffer size for string")]
    InvalidBufferSize,

    #[error("Failed to convert CFString to UTF-8")]
    ConvertString,

    #[error("UTF-8 conversion error: {0}")]
    UTF8ConversionError(#[from] Utf8Error),

    #[error("Failed to send initial event")]
    SendInitialEvent(#[from] std::sync::mpsc::SendError<Event>),
}
