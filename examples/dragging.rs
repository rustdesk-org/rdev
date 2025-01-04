use std::sync::atomic::AtomicBool;

use rdev::{listen, Event, EventType};

static DRAGGING: AtomicBool = AtomicBool::new(false);

fn callback(event: Event) {
    match event.event_type {
        EventType::MouseMove { x, y } => {
            if DRAGGING.load(std::sync::atomic::Ordering::Relaxed) {
                println!("Mouse dragged to ({}, {})", x, y);
            } else {
                println!("Mouse moved to ({}, {})", x, y);
            }
        }
        EventType::ButtonPress(button) => {
            println!("Mouse button pressed: {:?}", button);
            DRAGGING.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        EventType::ButtonRelease(button) => {
            println!("Mouse button released: {:?}", button);
            DRAGGING.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        _ => {
            println!("Unhandled Event: {:?}", event);
        }
    }
}

fn main() {
    if let Err(error) = listen(callback) {
        println!("Error: {:?}", error)
    }
}
