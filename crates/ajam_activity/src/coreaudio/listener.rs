use std::os::raw::c_void;
use std::slice;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::monitor::Event;

use super::property::{
    get_device_id, get_device_name, DEFAULT_INPUT_DEVICE_PROPERTY_ADDRESS,
    DEFAULT_OUTPUT_DEVICE_PROPERTY_ADDRESS,
};
use super::sys;

/// A context for the coreaudio listener.
struct ListenerContext {
    tx: Sender<Event>,
}

pub(crate) fn start_coreaudio_listener(tx: Sender<Event>) {
    let context = Box::new(ListenerContext { tx });
    let context_ptr = Box::into_raw(context) as *mut c_void;

    extern "C" fn handle_audio_device(
        id: sys::AudioObjectID,
        number_of_addresses: u32,
        addresses: *const sys::AudioObjectPropertyAddress,
        data: *mut c_void,
    ) -> sys::OSStatus {
        let addrs = unsafe { slice::from_raw_parts(addresses, number_of_addresses as usize) };

        for addr in addrs.iter() {
            let device_id = get_device_id(id, addr).unwrap();
            let Some(device_name) = get_device_name(device_id) else {
                continue;
            };

            let event = match addr.mSelector {
                sys::kAudioHardwarePropertyDefaultOutputDevice => {
                    Event::AudioOutputChange(device_name.clone())
                }
                sys::kAudioHardwarePropertyDefaultInputDevice => {
                    Event::AudioInputChange(device_name.clone())
                }
                _ => continue,
            };

            let context = unsafe { &*(data as *mut ListenerContext) };
            if let Err(e) = context.tx.send(event) {
                println!("Error sending event: {:?}", e);
            }
        }

        0 // sys::noErr.
    }

    let _ = audio_object_add_property_listener(
        sys::kAudioObjectSystemObject,
        &DEFAULT_OUTPUT_DEVICE_PROPERTY_ADDRESS,
        Some(handle_audio_device),
        context_ptr,
    );

    let _ = audio_object_add_property_listener(
        sys::kAudioObjectSystemObject,
        &DEFAULT_INPUT_DEVICE_PROPERTY_ADDRESS,
        Some(handle_audio_device),
        context_ptr,
    );

    loop {
        thread::sleep(Duration::from_millis(100));
    }
}

fn audio_object_add_property_listener(
    id: sys::AudioObjectID,
    address: &sys::AudioObjectPropertyAddress,
    listener: sys::AudioObjectPropertyListenerProc,
    data: *mut c_void,
) -> sys::OSStatus {
    unsafe { sys::AudioObjectAddPropertyListener(id, address, listener, data) }
}
