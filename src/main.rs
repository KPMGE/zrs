const ZOOM_WINDOW_WIDTH: u32 = 200;
const ZOOM_WINDOW_HEIGHT: u32 = 150;

mod xshape;
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

    let mut gwa: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
    let status = unsafe { xlib::XGetWindowAttributes(display, root, &mut gwa) };

    if status == 0 {
        panic!("ERROR: Failed to get window attributes!");
    }

    let zoom_window = unsafe {
        xlib::XCreateSimpleWindow(
            display,
            root,
            0,
            0,
            ZOOM_WINDOW_WIDTH,
            ZOOM_WINDOW_HEIGHT,
            1,
            xlib::XBlackPixel(display, 0),
            xlib::XWhitePixel(display, 0),
        )
    };

    let circular_mask =
        xshape::create_circular_mask(display, root, ZOOM_WINDOW_WIDTH, ZOOM_WINDOW_HEIGHT);

    unsafe {
        xshape::XShapeCombineMask(
            display,
            zoom_window,
            xshape::SHAPE_BOUNDING,
            0,
            0,
            circular_mask,
            xshape::SHAPE_SET,
        );
    }

    let status = unsafe { xlib::XMapWindow(display, zoom_window) };
    if status == 0 {
        panic!("ERROR: Cannot map window!");
    }

    let gc = unsafe {
        xlib::XCreateGC(
            display,
            zoom_window,
            0,
            std::ptr::null::<xlib::XGCValues>() as *mut x11::xlib::XGCValues,
        )
    };

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

    let (tx, rx) = std::sync::mpsc::channel();
    let zoom_factor = Arc::new(atomic::AtomicI8::new(2));
    let zoom_factor_clone = zoom_factor.clone();

    let mut event: xlib::XEvent = unsafe { std::mem::zeroed() };

    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                4 => {
                    let previous = zoom_factor_clone.load(atomic::Ordering::SeqCst);
                    let new = (previous + 1).clamp(1, 10);
                    zoom_factor_clone.store(new, atomic::Ordering::SeqCst);
                }
                5 => {
                    let previous = zoom_factor_clone.load(atomic::Ordering::SeqCst);
                    let new = (previous - 1).clamp(1, 10);
                    zoom_factor_clone.store(new, atomic::Ordering::SeqCst);
                }
                _ => {}
            }
        }
    });

    loop {
        let pending_events = unsafe { xlib::XPending(display) };

        if pending_events > 0 {
            unsafe { xlib::XNextEvent(display, &mut event) };

            match event.get_type() {
                xlib::KeyPress => {
                    let key_event = unsafe { event.key };
                    let keysym =
                        unsafe { xlib::XKeycodeToKeysym(display, key_event.keycode as _, 0) };

                    if keysym == x11::keysym::XK_Escape as u64 {
                        println!("Escape key pressed. Exiting...");
                        unsafe { xlib::XCloseDisplay(display) };
                        std::process::exit(0);
                    }
                }
                xlib::ButtonPress => {
                    let button_event = unsafe { event.button };
                    if let Err(_) = tx.send(button_event.button) {
                        break;
                    }
                }
                _ => {}
            }
        }

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
        let zoomed_image = scale_image(
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

fn scale_image(
    display: *mut xlib::Display,
    visual: *mut xlib::Visual,
    depth: i32,
    src_image: *mut xlib::XImage,
    new_width: i32,
    new_height: i32,
) -> *mut xlib::XImage {
    let scaled_image = unsafe {
        let image_size_in_bytes = new_width * new_height * 4;
        let data_ptr = libc::malloc(image_size_in_bytes as libc::size_t) as *mut i8;

        if data_ptr.is_null() {
            panic!("Failed to allocate memory for image data");
        }

        xlib::XCreateImage(
            display,
            visual,
            depth as u32,
            xlib::ZPixmap,
            0,
            data_ptr,
            new_width.try_into().unwrap(),
            new_height.try_into().unwrap(),
            32,
            0,
        )
    };

    if scaled_image.is_null() {
        panic!("Failed to create scaled XImage");
    }

    for y in 0..new_height {
        for x in 0..new_width {
            let src_x = (x * unsafe { (*src_image).width } / new_width)
                .min(unsafe { (*src_image).width - 1 });
            let src_y = (y * unsafe { (*src_image).height } / new_height)
                .min(unsafe { (*src_image).height - 1 });

            let pixel = unsafe { xlib::XGetPixel(src_image, src_x, src_y) };
            unsafe { xlib::XPutPixel(scaled_image, x, y, pixel) };
        }
    }

    scaled_image
}
