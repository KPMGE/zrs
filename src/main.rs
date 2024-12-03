const ZOOM_WINDOW_WIDTH: u32 = 200;
const ZOOM_WINDOW_HEIGHT: u32 = 150;

mod events;
mod setup;
mod xshape;
mod zoom;

use std::sync::{atomic, Arc};
use x11::xlib;

fn main() {
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };

    if display.is_null() {
        panic!("ERROR: Cannot open X display!");
    }

    let root = unsafe { xlib::XDefaultRootWindow(display) };
    if root == 0 {
        panic!("ERROR: Failed create deafault root window!");
    }

    setup::setup_x11_event_handlers(display, root);

    let mut gwa: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
    let status = unsafe { xlib::XGetWindowAttributes(display, root, &mut gwa) };

    if status == 0 {
        panic!("ERROR: Failed to get window attributes!");
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let zoom_factor = Arc::new(atomic::AtomicI8::new(2));
    let zoom_factor_clone = zoom_factor.clone();

    let mut event: xlib::XEvent = unsafe { std::mem::zeroed() };

    std::thread::spawn(move || {
        events::handle_zoom_mouse_events(rx, zoom_factor_clone);
    });

    let zoom_window =
        zoom::create_zoom_window(display, root, ZOOM_WINDOW_WIDTH, ZOOM_WINDOW_HEIGHT);

    let gc = unsafe {
        xlib::XCreateGC(
            display,
            zoom_window,
            0,
            std::ptr::null::<xlib::XGCValues>() as *mut x11::xlib::XGCValues,
        )
    };

    loop {
        events::handle_x11_events(display, &mut event, &tx);

        let mut mouse_x = 0;
        let mut mouse_y = 0;
        let mut root_return = 0;
        let mut child_return = 0;
        let mut win_x = 0;
        let mut win_y = 0;
        let mut mask_return = 0;

        let status = unsafe {
            xlib::XQueryPointer(
                display,
                root,
                &mut root_return,
                &mut child_return,
                &mut mouse_x,
                &mut mouse_y,
                &mut win_x,
                &mut win_y,
                &mut mask_return,
            )
        };

        if status == 0 {
            panic!("ERROR: Cannot query pointer!");
        }

        let factor = zoom_factor.load(atomic::Ordering::SeqCst) as i32;
        let capture_width = (ZOOM_WINDOW_WIDTH as i32 / factor).min(gwa.width);
        let capture_height = (ZOOM_WINDOW_HEIGHT as i32 / factor).min(gwa.height);

        let mut start_x = mouse_x - capture_width / 2;
        let mut start_y = mouse_y - capture_height / 2;

        start_x = start_x.clamp(0, gwa.width - capture_width);
        start_y = start_y.clamp(0, gwa.height - capture_height);

        // Capture the desktop image
        let desktop_image = unsafe {
            xlib::XGetImage(
                display,
                root,
                start_x,
                start_y,
                capture_width.try_into().unwrap(),
                capture_height.try_into().unwrap(),
                xlib::XAllPlanes(),
                xlib::ZPixmap,
            )
        };

        if desktop_image.is_null() {
            panic!("Failed to capture desktop image\n");
        }

        // // Scale the captured region to fit the zoom window
        let zoomed_image = zoom::scale_image(
            display,
            gwa.visual,
            gwa.depth,
            desktop_image,
            ZOOM_WINDOW_WIDTH as i32,
            ZOOM_WINDOW_HEIGHT as i32,
        );

        // Display the zoomed image in the window
        unsafe {
            xlib::XPutImage(
                display,
                zoom_window,
                gc,
                zoomed_image,
                0,
                0,
                0,
                0,
                ZOOM_WINDOW_WIDTH,
                ZOOM_WINDOW_HEIGHT,
            )
        };

        // Move window according to the mouse position
        let mut window_x = mouse_x - (ZOOM_WINDOW_WIDTH as i32 + 20);
        let mut window_y = mouse_y - (ZOOM_WINDOW_HEIGHT as i32 + 20);

        if mouse_y - ZOOM_WINDOW_HEIGHT as i32 <= 0 {
            window_y = mouse_y;
        }

        if mouse_x - ZOOM_WINDOW_WIDTH as i32 <= gwa.width {
            window_x = mouse_x;
        }

        if mouse_x + ZOOM_WINDOW_WIDTH as i32 >= gwa.width {
            window_x = mouse_x - ZOOM_WINDOW_WIDTH as i32;
        }

        // Move window according to mouse position
        unsafe {
            xlib::XMoveWindow(display, zoom_window, window_x, window_y);
        }

        // Cleanup
        unsafe {
            xlib::XDestroyImage(desktop_image);
            xlib::XDestroyImage(zoomed_image);
            xlib::XFlush(display);
        };

        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}
