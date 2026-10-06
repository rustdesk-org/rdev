#![allow(improper_ctypes_definitions)]
use crate::macos::common::*;
use crate::rdev::{Event, ListenError};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use core_graphics::{
    event::{CGEvent, CGEventTapLocation, CGEventType},
    sys::CGEventRef,
};
use foreign_types::ForeignType;
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::os::raw::c_void;

thread_local! {
    static GLOBAL_CALLBACK: RefCell<Option<Box<dyn FnMut(Event)>>> = RefCell::new(None);
}

fn callback_run_loop() -> CFRunLoopRef {
    unsafe { CFRunLoopGetCurrent() }
}

unsafe extern "C" fn raw_callback(
    _proxy: CGEventTapProxy,
    _type: CGEventType,
    cg_event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    if cg_event.is_null() {
        return cg_event;
    }
    let cg_event_ref = ManuallyDrop::new(CGEvent::from_ptr(cg_event));
    if let Ok(mut state) = KEYBOARD_STATE.lock() {
        if let Some(keyboard) = state.as_mut() {
            if let Some(event) = convert(_type, &cg_event_ref, keyboard) {
                GLOBAL_CALLBACK.with(|slot| {
                    if let Ok(mut callback) = slot.try_borrow_mut() {
                        if let Some(callback) = callback.as_mut() {
                            callback(event);
                        }
                    }
                });
            }
        }
    }
    // println!("Event ref END {:?}", cg_event_ptr);
    cg_event
}

pub fn listen<T>(callback: T) -> Result<(), ListenError>
where
    T: FnMut(Event) + 'static,
{
    let mut types = kCGEventMaskForAllEvents;
    if crate::keyboard_only() {
        types = (1 << CGEventType::KeyDown as u64)
            + (1 << CGEventType::KeyUp as u64)
            + (1 << CGEventType::FlagsChanged as u64);
    }
    unsafe {
        GLOBAL_CALLBACK.with(|slot| slot.replace(Some(Box::new(callback))));
        let _pool = NSAutoreleasePool::new(nil);
        let tap = CGEventTapCreate(
            CGEventTapLocation::HID, // HID, Session, AnnotatedSession,
            kCGHeadInsertEventTap,
            CGEventTapOption::ListenOnly,
            types,
            raw_callback,
            nil,
        );
        if tap.is_null() {
            return Err(ListenError::EventTapError);
        }
        let _loop = CFMachPortCreateRunLoopSource(nil, tap, 0);
        if _loop.is_null() {
            return Err(ListenError::LoopSourceError);
        }

        let current_loop = callback_run_loop();
        CFRunLoopAddSource(current_loop, _loop, kCFRunLoopCommonModes);

        CGEventTapEnable(tap, true);
        CFRunLoopRun();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_listener_uses_its_own_run_loop() {
        let main_loop = unsafe { CFRunLoopGetMain() } as usize;
        let worker_loop = std::thread::spawn(|| callback_run_loop() as usize)
            .join()
            .unwrap();

        assert_ne!(worker_loop, main_loop);
    }
}
