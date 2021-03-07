// This is the XCB example program from Wikipedia ported to xcb-sys:
// See https://en.wikipedia.org/w/index.php?title=XCB&oldid=1010746260#Example

use std::os::raw::c_void;
use std::ptr::{null, null_mut};
use xcb_sys::*;

fn main()
{
    unsafe {
        // open connection to the server
        let c = xcb_connect(null(), null_mut());
        if xcb_connection_has_error(c) != 0 {
            eprintln!("Cannot open display");
            std::process::exit(1);
        }

        // Get the first screen
        let s = xcb_setup_roots_iterator(xcb_get_setup(c)).data;

        // create black graphics context
        let g = xcb_generate_id(c);
        let w = (*s).root;
        let mask = XCB_GC_FOREGROUND | XCB_GC_GRAPHICS_EXPOSURES;
        let values = [(*s).black_pixel, 0];
        xcb_create_gc(c, g, w, mask, &values as *const u32 as *const c_void);

        // create window
        let w = xcb_generate_id(c);
        let mask = XCB_CW_BACK_PIXEL | XCB_CW_EVENT_MASK;
        let values = [(*s).white_pixel, XCB_EVENT_MASK_EXPOSURE | XCB_EVENT_MASK_KEY_PRESS];
        xcb_create_window(c, (*s).root_depth, w, (*s).root,
                10, 10, 100, 100, 1,
                XCB_WINDOW_CLASS_INPUT_OUTPUT as u16, (*s).root_visual,
                mask, &values as *const u32 as *const c_void);

        // map (show) the window
        xcb_map_window(c, w);

        xcb_flush(c);

        // event loop
        let mut done = false;
        while !done {
            let e = xcb_wait_for_event(c);
            if e.is_null() {
                break
            }

            match ((*e).response_type & !0x80) as u32 {
                XCB_EXPOSE => {
                    let r = xcb_rectangle_t {
                        x: 20,
                        y: 20,
                        width: 60,
                        height: 60,
                    };
                    xcb_poly_fill_rectangle(c, w, g, 1, &r);
                    xcb_flush(c);
                }
                XCB_KEY_PRESS => {
                    done = true;
                }
                _ => {}
            }
            free(e as *mut c_void);
        }

        // close connection to server
        xcb_disconnect(c);
    }
}

// No need to depend on the libc crate just for this
extern {
    fn free(p: *mut c_void);
}
