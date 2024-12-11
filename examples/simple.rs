use rdev::{listen, Event, EventType};

fn callback(event: Event) {
    match event.event_type {
        EventType::MouseMove { x, y } => println!("Mouse moved to ({}, {})", x, y),
        EventType::ButtonPress(button) => println!("Mouse button pressed: {:?}", button),
        EventType::ButtonRelease(button) => println!("Mouse button released: {:?}", button),
        EventType::KeyPress(key) => println!("Key pressed: {:?}", key),
        EventType::KeyRelease(key) => println!("Key released: {:?}", key),
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
