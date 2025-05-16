#![allow(non_snake_case, non_upper_case_globals)]

// https://developer.apple.com/documentation/coreaudio/1545886-anonymous/kaudiohardwarepropertydevices?language=objc
// https://github.com/phracker/MacOSX-SDKs/blob/9fc3ed0ad0345950ac25c28695b0427846eea966/MacOSX10.13.sdk/System/Library/Frameworks/CoreAudio.framework/Versions/A/Headers/AudioHardware.h#L585

use std::os::raw::{c_void, c_char};

pub type OSStatus = i32;

// OSStatus common values
pub const kAudioHardwareNoError: OSStatus = 0;
// pub const kAudioHardwareNotRunningError: OSStatus = -14;
// pub const kAudioHardwareUnspecifiedError: OSStatus = -14000;
// pub const kAudioHardwareUnknownPropertyError: OSStatus = -14001; // = 2003332927 when interpreted as u32
// pub const kAudioHardwareBadPropertySizeError: OSStatus = -14002;

// CoreFoundation типы
pub type CFIndex = isize;
pub type CFStringEncoding = u32;
pub const kCFStringEncodingUTF8: CFStringEncoding = 0x08000100;
pub type CFStringRef = *const c_void;
// pub const kCFStringEncodingInvalidId: CFStringEncoding = 0xffffffff;
// pub const kCFNotFound: CFIndex = -1;

pub type AudioObjectID = u32;

pub type AudioDeviceID = u32;

pub const kAudioObjectSystemObject: AudioObjectID = 1;

pub type AudioObjectPropertySelector = u32;

// // 0x'dev#' = 0x64657623 = 1684370979
// pub const kAudioHardwarePropertyDevices: AudioObjectPropertySelector = 1684370979;

// 0x'dIn ' = 0x64496E20
pub const kAudioHardwarePropertyDefaultInputDevice: AudioObjectPropertySelector = 0x64496E20;

// 0x'dOut' = 0x644F7574
pub const kAudioHardwarePropertyDefaultOutputDevice: AudioObjectPropertySelector = 0x644F7574;

// 0x'lnam' = 0x6C6E616D = 1819173997
pub const kAudioObjectPropertyName: AudioObjectPropertySelector = 0x6c6e616d;

// // 0x'name' = 0x6e616d65 = 1852798309
// pub const kAudioDevicePropertyDeviceNameCFString: AudioObjectPropertySelector = 1852798309;

// // 0x'dIn ' = 0x64496E20
// pub const kAudioDevicePropertyDefaultInputDevice: AudioObjectPropertySelector = 0x64496E20;

// // 0x'dOut' = 0x644F7574
// pub const kAudioDevicePropertyDefaultOutputDevice: AudioObjectPropertySelector = 0x644F7574;

pub type AudioObjectPropertyScope = u32;
// https://developer.apple.com/documentation/coreaudio/1494464-anonymous/kaudioobjectpropertyscopeglobal
// https://github.com/phracker/MacOSX-SDKs/blob/9fc3ed0ad0345950ac25c28695b0427846eea966/MacOSX10.13.sdk/System/Library/Frameworks/CoreAudio.framework/Versions/A/Headers/AudioHardwareBase.h#L198
// 0x'glob' = 0x676C6F62 = 1735159650
pub const kAudioObjectPropertyScopeGlobal: AudioObjectPropertyScope = 1735159650;

pub type AudioObjectPropertyElement = u32;
// https://developer.apple.com/documentation/coreaudio/1494464-anonymous/kaudioobjectpropertyelementmaster
// https://github.com/phracker/MacOSX-SDKs/blob/9fc3ed0ad0345950ac25c28695b0427846eea966/MacOSX10.13.sdk/System/Library/Frameworks/CoreAudio.framework/Versions/A/Headers/AudioHardwareBase.h#L202
pub const kAudioObjectPropertyElementMaster: AudioObjectPropertyElement = 0;

#[repr(C)]
pub struct AudioObjectPropertyAddress {
    pub mSelector: AudioObjectPropertySelector,
    pub mScope: AudioObjectPropertyScope,
    pub mElement: AudioObjectPropertyElement,
}

// https://developer.apple.com/documentation/coreaudio/audioobjectpropertylistenerproc?language=objc
// https://github.com/phracker/MacOSX-SDKs/blob/9fc3ed0ad0345950ac25c28695b0427846eea966/MacOSX10.13.sdk/System/Library/Frameworks/CoreAudio.framework/Versions/A/Headers/AudioHardware.h#L117-L143
pub type AudioObjectPropertyListenerProc = Option<unsafe extern "C" fn(
    AudioObjectID,
    u32,
    *const AudioObjectPropertyAddress,
    *mut c_void
) -> OSStatus>;

#[link(name = "CoreAudio", kind = "framework")] // Link to a dynamic library in CoreAudio framework.
extern "C" {
    // https://developer.apple.com/documentation/coreaudio/1422472-audioobjectaddpropertylistener?language=objc
    // https://github.com/phracker/MacOSX-SDKs/blob/9fc3ed0ad0345950ac25c28695b0427846eea966/MacOSX10.13.sdk/System/Library/Frameworks/CoreAudio.framework/Versions/A/Headers/AudioHardware.h#L331-L350
    pub fn AudioObjectAddPropertyListener(
        id: AudioObjectID,
        address: *const AudioObjectPropertyAddress,
        listener: AudioObjectPropertyListenerProc,
        data: *mut c_void,
    ) -> OSStatus;
    
    pub fn AudioObjectGetPropertyDataSize(
        id: AudioObjectID,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        data_size: *mut u32,
    ) -> OSStatus;
    
    pub fn AudioObjectGetPropertyData(
        id: AudioObjectID,
        address: *const AudioObjectPropertyAddress,
        qualifier_data_size: u32,
        qualifier_data: *const c_void,
        data_size: *mut u32,
        data: *mut c_void,
    ) -> OSStatus;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFStringGetLength(string: CFStringRef) -> CFIndex;
    pub fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> bool;
    pub fn CFStringGetMaximumSizeForEncoding(
        length: CFIndex,
        encoding: CFStringEncoding,
    ) -> CFIndex;
    pub fn CFRelease(cf: CFStringRef);
}
