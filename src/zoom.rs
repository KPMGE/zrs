use crate::xshape;
use x11::xlib;

pub fn scale_image(
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

pub fn create_zoom_window(display: *mut xlib::Display, root: u64, width: u32, height: u32) -> u64 {
    let zoom_window = unsafe {
        xlib::XCreateSimpleWindow(
            display,
            root,
            0,
            0,
            width,
            height,
            1,
            xlib::XBlackPixel(display, 0),
            xlib::XWhitePixel(display, 0),
        )
    };

    let circular_mask = xshape::create_circular_mask(display, root, width, height);

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

    zoom_window
}
