use std::os::raw::c_int;
use x11::xlib;

#[link(name = "Xext")]
extern "C" {
    pub fn XShapeCombineMask(
        display: *mut x11::xlib::Display,
        dest: x11::xlib::Window,
        dest_kind: c_int,
        x_offset: c_int,
        y_offset: c_int,
        src: x11::xlib::Pixmap,
        op: c_int,
    );
}

pub const SHAPE_BOUNDING: c_int = 0;
pub const SHAPE_SET: c_int = 0;

pub fn create_circular_mask(
    display: *mut xlib::Display,
    root: xlib::Window,
    width: u32,
    height: u32,
) -> xlib::Pixmap {
    unsafe {
        // Create a pixmap for the mask
        let mask = xlib::XCreatePixmap(display, root, width, height, 1);
        let gc = xlib::XCreateGC(display, mask, 0, std::ptr::null_mut());

        // Fill the mask with a transparent background
        xlib::XSetForeground(display, gc, 0);
        xlib::XFillRectangle(display, mask, gc, 0, 0, width, height);

        // Draw a solid circle
        xlib::XSetForeground(display, gc, 1);
        xlib::XFillArc(display, mask, gc, 0, 0, width, height, 0, 360 * 64);

        // Free the graphics context
        xlib::XFreeGC(display, gc);
        mask
    }
}
