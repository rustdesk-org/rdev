#![allow(improper_ctypes_definitions)]
use crate::macos::common::*;
use crate::rdev::{Event, GrabError};
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
    static GLOBAL_CALLBACK: RefCell<Option<Box<dyn FnMut(Event) -> Option<Event>>>> =
        RefCell::new(None);
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
                            if callback(event).is_none() {
                                cg_event_ref.set_type(CGEventType::Null);
                            }
                        }
                    }
                });
            }
        }
    }
    cg_event
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    #[test]
    fn callback_borrows_the_system_event() {
        let _: QCallback = raw_callback;
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
        let event = CGEvent::new(source).unwrap();
        let event_ptr = event.as_ptr();

        let returned = unsafe {
            raw_callback(
                nil,
                CGEventType::MouseMoved,
                event_ptr,
                std::ptr::null_mut(),
            )
        };

        assert_eq!(returned, event_ptr);
        assert!(!event.location().x.is_nan());
    }
}

static mut CUR_LOOP: CFRunLoopSourceRef = std::ptr::null_mut();

#[inline]
pub fn is_grabbed() -> bool {
    unsafe {
        !CUR_LOOP.is_null()
    }
}

pub fn grab<T>(callback: T) -> Result<(), GrabError>
where
    T: FnMut(Event) -> Option<Event> + 'static,
{
    if is_grabbed() {
        return Ok(());
    }

    unsafe {
        GLOBAL_CALLBACK.with(|slot| slot.replace(Some(Box::new(callback))));
        let _pool = NSAutoreleasePool::new(nil);
        let tap = CGEventTapCreate(
            CGEventTapLocation::Session, // HID, Session, AnnotatedSession,
            kCGHeadInsertEventTap,
            CGEventTapOption::Default,
            kCGEventMaskForAllEvents,
            raw_callback,
            nil,
        );
        if tap.is_null() {
            return Err(GrabError::EventTapError);
        }
        let _loop = CFMachPortCreateRunLoopSource(nil, tap, 0);
        if _loop.is_null() {
            return Err(GrabError::LoopSourceError);
        }

        CUR_LOOP = CFRunLoopGetCurrent() as _;
        CFRunLoopAddSource(CUR_LOOP, _loop, kCFRunLoopCommonModes);

        CGEventTapEnable(tap, true);
        CFRunLoopRun();
    }
    Ok(())
}

pub fn exit_grab() -> Result<(), GrabError> {
    unsafe {
        if !CUR_LOOP.is_null() {
            CFRunLoopStop(CUR_LOOP);
            CUR_LOOP = std::ptr::null_mut();
        }
    }
    Ok(())
}
