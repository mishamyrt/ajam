#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use cocoa::appkit::NSApplicationActivationPolicy;
use cocoa::base::{id, YES};
use core_foundation::array::CFArray;
use core_foundation::string::CFString;
use objc::runtime::{Object, BOOL};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;
use std::os::raw::{c_char, c_uint};
use std::sync::mpsc::{self, Receiver, Sender};

pub type CGConnectionID = c_uint;

#[derive(Debug, Clone)]
pub enum ActivityEvent {
    AppChange(String),
}

pub struct ActivityMonitor {
    event_tx: Sender<ActivityEvent>,
}

#[allow(unexpected_cfgs, improper_ctypes)]
impl ActivityMonitor {
    pub fn new() -> (Self, Receiver<ActivityEvent>) {
        let (tx, rx) = mpsc::channel();
        (ActivityMonitor { event_tx: tx }, rx)
    }

    pub fn start_listening(self) {
        AppDelegate::new(self.event_tx).start_listening();
    }

    pub fn start_listening_background(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            AppDelegate::new(self.event_tx).start_listening();
        })
    }

    pub fn get_current_space_number() -> i32 {
        unsafe {
            let conn = CGSMainConnectionID();
            let displays = CGSCopyManagedDisplaySpaces(conn);
            let active_display = CGSCopyActiveMenuBarDisplayIdentifier(conn);
            let displays_array: id = msg_send![class!(NSArray), arrayWithArray:displays];

            let mut active_space_id = -1;
            let mut all_spaces = Vec::new();

            let count: usize = msg_send![displays_array, count];
            for i in 0..count {
                let display: id = msg_send![displays_array, objectAtIndex:i];

                let current_space = make_nsstring("Current Space");
                let spaces = make_nsstring("Spaces");
                let disp_id = make_nsstring("Display Identifier");

                let current: id = msg_send![display, objectForKey:current_space];
                let spaces_arr: id = msg_send![display, objectForKey:spaces];
                let disp_identifier: id = msg_send![display, objectForKey:disp_id];

                if current.is_null() || spaces_arr.is_null() || disp_identifier.is_null() {
                    continue;
                }

                let disp_str: id = msg_send![disp_identifier, description];
                let main_str = make_nsstring("Main");
                let active_str = make_nsstring(&active_display.to_string());
                let is_main: BOOL = msg_send![disp_str, isEqualToString:main_str];
                let is_active: BOOL = msg_send![disp_str, isEqualToString:active_str];

                if is_main == YES || is_active == YES {
                    let space_id_key = make_nsstring("ManagedSpaceID");
                    active_space_id = msg_send![current, objectForKey:space_id_key];
                }

                let spaces_count: usize = msg_send![spaces_arr, count];
                for j in 0..spaces_count {
                    let space: id = msg_send![spaces_arr, objectAtIndex:j];
                    let tile_key = make_nsstring("TileLayoutManager");
                    let tile_layout: id = msg_send![space, objectForKey:tile_key];

                    if tile_layout.is_null() {
                        all_spaces.push(space);
                    }
                }
            }

            if active_space_id == -1 {
                return -1;
            }

            for (index, space) in all_spaces.iter().enumerate() {
                let space_id_key = make_nsstring("ManagedSpaceID");
                let space_id: i32 = msg_send![*space, objectForKey:space_id_key];
                let space_number = index + 1;

                if space_id == active_space_id {
                    return space_number as i32;
                }
            }
            -1
        }
    }
}

#[allow(improper_ctypes)]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGSMainConnectionID() -> CGConnectionID;
    fn CGSCopyManagedDisplaySpaces(connection: CGConnectionID) -> CFArray;
    fn CGSCopyActiveMenuBarDisplayIdentifier(connection: CGConnectionID) -> CFString;
}

#[allow(unexpected_cfgs, improper_ctypes)]
unsafe fn make_nsstring(string: &str) -> id {
    let cls = class!(NSString);
    let string = std::ffi::CString::new(string).unwrap();
    msg_send![cls, stringWithUTF8String:string.as_ptr()]
}

struct AppState {
    event_tx: Sender<ActivityEvent>,
}

#[allow(improper_ctypes, unexpected_cfgs)]
impl AppState {
    fn new(event_tx: Sender<ActivityEvent>) -> Self {
        AppState { event_tx }
    }

    fn notify_active_app(&self, notification: id) {
        unsafe {
            let user_info: id = msg_send![notification, userInfo];
            if user_info.is_null() {
                return;
            }

            let app_key = make_nsstring("NSWorkspaceApplicationKey");
            let app: id = msg_send![user_info, objectForKey:app_key];
            if app.is_null() {
                return;
            }

            let bundle_id: id = msg_send![app, bundleIdentifier];
            if bundle_id.is_null() {
                return;
            }

            let utf8: *const c_char = msg_send![bundle_id, UTF8String];
            if utf8.is_null() {
                return;
            }

            let cstr = std::ffi::CStr::from_ptr(utf8);
            match cstr.to_str() {
                Ok(bundle_str) => {
                    if let Err(e) = self
                        .event_tx
                        .send(ActivityEvent::AppChange(bundle_str.to_string()))
                    {
                        println!("❌ Ошибка отправки события: {}", e);
                    }
                }
                Err(e) => {
                    println!("❌ Ошибка конвертации строки: {}", e);
                }
            }
        }
    }

    fn setup_notifications(&self, delegate: id) {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let frontmost_app: id = msg_send![workspace, frontmostApplication];
            if !frontmost_app.is_null() {
                let bundle_id: id = msg_send![frontmost_app, bundleIdentifier];
                if !bundle_id.is_null() {
                    let utf8: *const c_char = msg_send![bundle_id, UTF8String];
                    if !utf8.is_null() {
                        let cstr = std::ffi::CStr::from_ptr(utf8);
                        if let Ok(bundle_str) = cstr.to_str() {
                            if let Err(e) = self
                                .event_tx
                                .send(ActivityEvent::AppChange(bundle_str.to_string()))
                            {
                                println!("❌ Ошибка отправки события: {}", e);
                            }
                        }
                    }
                }
            }

            let workspace_notification_center: id = msg_send![workspace, notificationCenter];
            let app_active = make_nsstring("NSWorkspaceDidActivateApplicationNotification");

            let _: () = msg_send![workspace_notification_center,
                addObserver:delegate
                selector:sel!(updateActiveApplication:)
                name:app_active
                object:workspace];
        }
    }
}

struct AppDelegate {
    _delegate: id,
}

#[allow(improper_ctypes, unexpected_cfgs)]
impl AppDelegate {
    fn new(event_tx: Sender<ActivityEvent>) -> Self {
        unsafe {
            let mut decl =
                objc::declare::ClassDecl::new("RustAppDelegate", class!(NSObject)).unwrap();

            decl.add_ivar::<*mut c_void>("_rustState");

            extern "C" fn update_active_application(
                this: &Object,
                _sel: objc::runtime::Sel,
                notification: id,
            ) {
                unsafe {
                    let state_ptr: *mut c_void = *this.get_ivar("_rustState");
                    let state = &*(state_ptr as *const AppState);
                    state.notify_active_app(notification);
                }
            }

            decl.add_method(
                sel!(updateActiveApplication:),
                update_active_application as extern "C" fn(&Object, _, _),
            );

            decl.register();

            let delegate_class = class!(RustAppDelegate);
            let delegate: id = msg_send![delegate_class, new];

            let state = Box::new(AppState::new(event_tx));
            let state_ptr = Box::into_raw(state) as *mut c_void;
            (*delegate).set_ivar("_rustState", state_ptr);

            let state = &*(state_ptr as *const AppState);
            state.setup_notifications(delegate);

            AppDelegate {
                _delegate: delegate,
            }
        }
    }

    fn setup_application(&self) {
        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setActivationPolicy:
                NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory];
        }
    }

    fn setup_observers(&self) {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let notification_center: id = msg_send![workspace, notificationCenter];
            let active_app_name = make_nsstring("NSWorkspaceDidActivateApplicationNotification");

            let _: () = msg_send![notification_center,
                addObserver:self._delegate
                selector:sel!(updateActiveApplication:)
                name:active_app_name
                object:workspace];
        }
    }

    fn start_listening(self) {
        self.setup_application();
        self.setup_observers();

        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setDelegate:self._delegate];
            let _: () = msg_send![app, run];
        }
    }
}
