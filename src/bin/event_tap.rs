//! CGEventTap-based Surface Dial volume controller for macOS
//!
//! The Surface Dial sends:
//! - Button press: OtherMouseDown/Up with button 31
//! - Rotation: KeyDown/Up with keycode 123 (left) or 124 (right)

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType, EventField,
};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const KEYCODE_LEFT_ARROW: i64 = 123;
const KEYCODE_RIGHT_ARROW: i64 = 124;
const VOLUME_STEP: i32 = 2;

static LAST_BUTTON_STATE: AtomicBool = AtomicBool::new(false);

fn get_volume() -> Option<i32> {
    let output = Command::new("osascript")
        .args(["-e", "output volume of (get volume settings)"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn set_volume(vol: i32) {
    let vol = vol.clamp(0, 100);
    let _ = Command::new("osascript")
        .args(["-e", &format!("set volume output volume {}", vol)])
        .output();
    println!("Volume: {}%", vol);
}

fn toggle_mute() {
    let _ = Command::new("osascript")
        .args(["-e", "set volume output muted not (output muted of (get volume settings))"])
        .output();
    println!("Toggled mute");
}

fn main() {
    println!("Surface Dial Volume Controller");
    println!("===============================");
    println!();
    println!("Rotate dial = adjust volume");
    println!("Press button = toggle mute");
    println!("Press Ctrl+C to exit.");
    println!();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        r.store(false, Ordering::SeqCst);
        CFRunLoop::get_current().stop();
    }).unwrap();

    let event_types = vec![
        CGEventType::KeyDown,
        CGEventType::OtherMouseDown,
        CGEventType::OtherMouseUp,
    ];

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        event_types,
        |_proxy, event_type, event| {
            match event_type {
                CGEventType::KeyDown => {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);

                    match keycode {
                        KEYCODE_LEFT_ARROW => {
                            // Rotate left = volume down
                            if let Some(current) = get_volume() {
                                let new_vol = (current - VOLUME_STEP).max(0);
                                if new_vol != current {
                                    set_volume(new_vol);
                                }
                            }
                        }
                        KEYCODE_RIGHT_ARROW => {
                            // Rotate right = volume up
                            if let Some(current) = get_volume() {
                                let new_vol = (current + VOLUME_STEP).min(100);
                                if new_vol != current {
                                    set_volume(new_vol);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                CGEventType::OtherMouseDown => {
                    let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                    if button == 31 {
                        let was_pressed = LAST_BUTTON_STATE.swap(true, Ordering::SeqCst);
                        if !was_pressed {
                            toggle_mute();
                        }
                    }
                }
                CGEventType::OtherMouseUp => {
                    let button = event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                    if button == 31 {
                        LAST_BUTTON_STATE.store(false, Ordering::SeqCst);
                    }
                }
                _ => {}
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
            println!("Listening for Surface Dial events...\n");
            CFRunLoop::run_current();
        }
        Err(()) => {
            eprintln!("Failed to create event tap!");
            eprintln!("Go to: System Settings > Privacy & Security > Accessibility");
            eprintln!("Add your terminal app.");
        }
    }
}
