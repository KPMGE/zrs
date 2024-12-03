use std::sync::{atomic, mpsc, Arc};
use x11::xlib;

const MAX_ZOOM_FACTOR: i8 = 20;

pub fn handle_zoom_mouse_events(rx: mpsc::Receiver<u32>, zoom_factor: Arc<atomic::AtomicI8>) {
    while let Ok(event) = rx.recv() {
        match event {
            4 => {
                let previous = zoom_factor.load(atomic::Ordering::SeqCst);
                let new = (previous + 1).clamp(1, MAX_ZOOM_FACTOR);
                zoom_factor.store(new, atomic::Ordering::SeqCst);
            }
            5 => {
                let previous = zoom_factor.load(atomic::Ordering::SeqCst);
                let new = (previous - 1).clamp(1, MAX_ZOOM_FACTOR);
                zoom_factor.store(new, atomic::Ordering::SeqCst);
            }
            _ => {}
        }
    }
}

pub fn handle_x11_events(
    display: *mut xlib::Display,
    event: &mut xlib::XEvent,
    tx: &std::sync::mpsc::Sender<u32>,
) {
    let pending_events = unsafe { xlib::XPending(display) };

    if pending_events > 0 {
        unsafe { xlib::XNextEvent(display, event) };

        match event.get_type() {
            xlib::KeyPress => {
                let key_event = unsafe { event.key };
                let keysym = unsafe { xlib::XKeycodeToKeysym(display, key_event.keycode as _, 0) };

                if keysym == x11::keysym::XK_Escape as u64 {
                    println!("Escape key pressed. Exiting...");
                    unsafe { xlib::XCloseDisplay(display) };
                    std::process::exit(0);
                }
            }
            xlib::ButtonPress => {
                let button_event = unsafe { event.button };
                if let Err(_) = tx.send(button_event.button) {
                    panic!("error whilw dingdi");
                }
            }
            _ => {}
        }
    }
}
