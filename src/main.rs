const ZOOM_WINDOW_WIDTH: u32 = 200;
const ZOOM_WINDOW_HEIGHT: u32 = 150;

mod xshape;
use x11::xlib;

fn main() {
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };

    if display.is_null() {
        panic!("Cannot open X display!");
    }

    let root = unsafe { xlib::XDefaultRootWindow(display) };
    let mut gwa: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
    let status = unsafe { xlib::XGetWindowAttributes(display, root, &mut gwa) };

    if status == 0 {
        eprintln!("Error: Failed to get window attributes.");
    } else {
        println!(
            "Window attributes retrieved: width = {}, height = {}",
            gwa.width, gwa.height
        );
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

    unsafe { xlib::XMapWindow(display, zoom_window) };
    let gc = unsafe {
        xlib::XCreateGC(
            display,
            zoom_window,
            0,
            std::ptr::null::<xlib::XGCValues>() as *mut x11::xlib::XGCValues,
        )
    };

    loop {
        let mut mouse_x = 0;
        let mut mouse_y = 0;
        let mut root_return = 0;
        let mut child_return = 0;
        let mut win_x = 0;
        let mut win_y = 0;
        let mut mask_return = 0;

        unsafe {
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
            );
        }

        let zoom_factor = 2;

        let capture_width = (ZOOM_WINDOW_WIDTH as i32 / zoom_factor).min(gwa.width);
        let capture_height = (ZOOM_WINDOW_HEIGHT as i32 / zoom_factor).min(gwa.height);

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
        let mut window_x = mouse_x - ZOOM_WINDOW_WIDTH as i32;
        let mut window_y = mouse_y - ZOOM_WINDOW_HEIGHT as i32;

        if mouse_y - ZOOM_WINDOW_HEIGHT as i32 <= 0 {
            window_y = mouse_y;
        }

        if mouse_x - ZOOM_WINDOW_WIDTH as i32 <= gwa.width {
            window_x = mouse_x;
        }

        if mouse_x + ZOOM_WINDOW_WIDTH as i32 >= gwa.width {
            window_x = mouse_x - ZOOM_WINDOW_WIDTH as i32;
        }

        unsafe {
            xlib::XMoveWindow(display, zoom_window, window_x, window_y);
        }

        // Cleanup
        unsafe {
            xlib::XDestroyImage(desktop_image);
            xlib::XDestroyImage(zoomed_image);
            xlib::XFlush(display);
        };

        // small delay for smoother updates (adjust as needed)
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
