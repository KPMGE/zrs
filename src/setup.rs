use x11::xlib;

pub fn setup_x11_event_handlers(display: *mut xlib::Display, root: u64) {
    // Grab the Escape key
    let status = unsafe {
        xlib::XGrabKey(
            display,
            xlib::XKeysymToKeycode(display, x11::keysym::XK_Escape as u64) as i32,
            0,
            root,
            1,
            xlib::GrabModeAsync,
            xlib::GrabModeAsync,
        )
    };

    if status == 0 {
        panic!("ERROR: Cannot gray ESC key!");
    }

    // Attempt to grab the pointer
    let status = unsafe {
        xlib::XGrabPointer(
            display,
            root,
            xlib::False,
            xlib::ButtonPressMask as u32,
            xlib::GrabModeAsync,
            xlib::GrabModeAsync,
            root,
            0,
            xlib::CurrentTime,
        )
    };

    if status != xlib::GrabSuccess {
        panic!("Failed to grab pointer!");
    }
}
