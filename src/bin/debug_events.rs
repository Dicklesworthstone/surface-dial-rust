//! Debug tool to see ALL CGEvents coming through
//! This will help identify what events (if any) the Surface Dial generates

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    println!("=== CGEvent Debug Tool ===");
    println!("Logging ALL events. Press Ctrl+C to exit.\n");
    println!("Try: keyboard, mouse, AND the Surface Dial.\n");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        CFRunLoop::get_current().stop();
    }).unwrap();

    // Listen to ALL event types
    let event_types = vec![
        CGEventType::Null,
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseUp,
        CGEventType::RightMouseDown,
        CGEventType::RightMouseUp,
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDragged,
        CGEventType::RightMouseDragged,
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::FlagsChanged,
        CGEventType::ScrollWheel,
        CGEventType::TabletPointer,
        CGEventType::TabletProximity,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
        CGEventType::OtherMouseDragged,
    ];

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        event_types,
        |_proxy, event_type, event| {
            // Get some common fields
            let field_87 = event.get_integer_value_field(87); // IORegistryEntryID

            match event_type {
                CGEventType::KeyDown | CGEventType::KeyUp => {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                    println!("[{:?}] keycode={} field87={}", event_type, keycode, field_87);
                }
                CGEventType::ScrollWheel => {
                    println!("[ScrollWheel] field87={}", field_87);
                }
                CGEventType::OtherMouseDown | CGEventType::OtherMouseUp => {
                    let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                    println!("[{:?}] button={} field87={}", event_type, button, field_87);
                }
                CGEventType::MouseMoved | CGEventType::LeftMouseDragged | CGEventType::RightMouseDragged => {
                    // Skip mouse movement spam
                }
                _ => {
                    println!("[{:?}] field87={}", event_type, field_87);
                }
            }
            Some(event.to_owned())
        },
    );

    match tap {
        Ok(tap) => {
            let source = tap.mach_port.create_runloop_source(0).unwrap();
            let run_loop = CFRunLoop::get_current();
            run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
            tap.enable();
            println!("Event tap active. Waiting for events...\n");
            CFRunLoop::run_current();
        }
        Err(()) => {
            eprintln!("Failed to create event tap!");
            eprintln!("Go to: System Settings > Privacy & Security > Input Monitoring");
            eprintln!("Add your terminal app.");
        }
    }
}
