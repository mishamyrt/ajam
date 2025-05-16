use std::ffi::CStr;
use std::os::raw::c_void;
use std::{mem, ptr};

use super::{sys, CoreAudioError};

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

pub(crate) fn get_active_device_name(
    address: &sys::AudioObjectPropertyAddress,
) -> Result<String, CoreAudioError> {
    let device_id = get_device_id(sys::kAudioObjectSystemObject, address)?;
    get_device_name(device_id)
}

pub(crate) fn get_device_id(
    object_id: sys::AudioObjectID,
    address: &sys::AudioObjectPropertyAddress,
) -> Result<sys::AudioDeviceID, CoreAudioError> {
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
        return Err(CoreAudioError::ReadProperty {
            selector: address.mSelector,
            status,
        });
    }

    Ok(device_id)
}

pub(crate) fn get_device_name(device_id: sys::AudioDeviceID) -> Result<String, CoreAudioError> {
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
        return Err(CoreAudioError::GetPropertySize {
            selector: address.mSelector,
            size,
        });
    }

    if size == 0 {
        return Err(CoreAudioError::EmptyProperty);
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
        return Err(CoreAudioError::ReadProperty {
            selector: address.mSelector,
            status,
        });
    }

    unsafe {
        let cf_length = sys::CFStringGetLength(cf_string_ref);
        
        let buffer_size = sys::CFStringGetMaximumSizeForEncoding(cf_length, sys::kCFStringEncodingUTF8);

        if buffer_size <= 0 {
            sys::CFRelease(cf_string_ref);
            return Err(CoreAudioError::InvalidBufferSize);
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
            return Err(CoreAudioError::ConvertString);
        }
        
        let c_str = CStr::from_ptr(buffer.as_ptr());
        let name = c_str.to_str()?.to_owned();
        Ok(name)
    }
}
