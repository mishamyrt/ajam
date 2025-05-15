use std::ffi::CStr;
use std::os::raw::c_void;
use std::{mem, ptr};

use super::sys;

pub(crate) const DEFAULT_OUTPUT_DEVICE_PROPERTY_ADDRESS: sys::AudioObjectPropertyAddress =
    sys::AudioObjectPropertyAddress {
        mSelector: sys::kAudioHardwarePropertyDefaultOutputDevice,
        mScope: sys::kAudioObjectPropertyScopeGlobal,
        mElement: sys::kAudioObjectPropertyElementMaster,
    };

pub(crate) const DEFAULT_INPUT_DEVICE_PROPERTY_ADDRESS: sys::AudioObjectPropertyAddress =
    sys::AudioObjectPropertyAddress {
        mSelector: sys::kAudioHardwarePropertyDefaultInputDevice,
        mScope: sys::kAudioObjectPropertyScopeGlobal,
        mElement: sys::kAudioObjectPropertyElementMaster,
    };

pub(crate) fn get_device_id(
    object_id: sys::AudioObjectID,
    address: &sys::AudioObjectPropertyAddress,
) -> Option<sys::AudioDeviceID> {
    let mut device_id: sys::AudioDeviceID = 0;
    let mut data_size = mem::size_of::<sys::AudioDeviceID>() as u32;

    let status = unsafe {
        sys::AudioObjectGetPropertyData(
            object_id,
            address as *const _,
            0,
            ptr::null(),
            &mut data_size as *mut _,
            &mut device_id as *mut _ as *mut c_void,
        )
    };

    if status != sys::kAudioHardwareNoError {
        println!("Error getting property data: {}", status);
        return None;
    }

    Some(device_id)
}

pub(crate) fn get_device_name(device_id: sys::AudioDeviceID) -> Option<String> {
    let address = sys::AudioObjectPropertyAddress {
        mSelector: sys::kAudioObjectPropertyName,
        mScope: sys::kAudioObjectPropertyScopeGlobal,
        mElement: sys::kAudioObjectPropertyElementMaster,
    };

    let mut size: u32 = 0;
    let status = unsafe {
        sys::AudioObjectGetPropertyDataSize(
            device_id,
            &address as *const _,
            0,
            ptr::null(),
            &mut size as *mut _,
        )
    };

    if status != sys::kAudioHardwareNoError {
        match status {
            s if s == sys::kAudioHardwareUnknownPropertyError => {
                println!(
                    "Error: Unknown property (kAudioHardwareUnknownPropertyError: {})",
                    status
                );
            }
            s if s == sys::kAudioHardwareBadPropertySizeError => {
                println!(
                    "Error: Bad property size (kAudioHardwareBadPropertySizeError: {})",
                    status
                );
            }
            s if s == sys::kAudioHardwareUnspecifiedError => {
                println!(
                    "Error: Unspecified error (kAudioHardwareUnspecifiedError: {})",
                    status
                );
            }
            _ => {
                println!("Error getting property size: {}", status);
            }
        }
        return None;
    }

    if size == 0 {
        println!("Property size is 0");
        return None;
    }

    let mut cf_string_ref: sys::CFStringRef = ptr::null();
    let mut data_size = size;

    let status = unsafe {
        sys::AudioObjectGetPropertyData(
            device_id,
            &address as *const _,
            0,
            ptr::null(),
            &mut data_size as *mut _,
            &mut cf_string_ref as *mut _ as *mut c_void,
        )
    };

    if status != sys::kAudioHardwareNoError || cf_string_ref.is_null() {
        println!("Error getting property data: {}", status);
        return None;
    }

    unsafe {
        let cf_length = sys::CFStringGetLength(cf_string_ref);
        
        let buffer_size = sys::CFStringGetMaximumSizeForEncoding(cf_length, sys::kCFStringEncodingUTF8);
        
        if buffer_size <= 0 {
            println!("Error: invalid buffer size for string");
            sys::CFRelease(cf_string_ref);
            return None;
        }
        
        let mut buffer = vec![0i8; (buffer_size + 1) as usize];
        
        let success = sys::CFStringGetCString(
            cf_string_ref, 
            buffer.as_mut_ptr(),
            buffer_size + 1,
            sys::kCFStringEncodingUTF8
        );
        
        sys::CFRelease(cf_string_ref);
        
        if !success {
            println!("Failed to convert CFString to UTF-8");
            return None;
        }
        
        let c_str = CStr::from_ptr(buffer.as_ptr());
        match c_str.to_str() {
            Ok(string) => Some(string.to_owned()),
            Err(e) => {
                println!("String conversion error: {}", e);
                None
            }
        }
    }
}
