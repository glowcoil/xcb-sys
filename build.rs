use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=xcb");

    let mut headers = Vec::new();
    headers.push("xcb");

    #[cfg(feature = "bigreq")]
    headers.push("bigreq");

    #[cfg(feature = "composite")]
    headers.push("composite");

    #[cfg(feature = "damage")]
    headers.push("damage");

    #[cfg(feature = "dpms")]
    headers.push("dpms");

    #[cfg(feature = "dri2")]
    headers.push("dri2");

    #[cfg(feature = "dri3")]
    headers.push("dri3");

    #[cfg(feature = "ewmh")]
    headers.push("xcb_ewmh");

    #[cfg(feature = "ge")]
    headers.push("ge");

    #[cfg(feature = "glx")]
    headers.push("glx");

    #[cfg(feature = "icccm")]
    headers.push("xcb_icccm");

    #[cfg(feature = "image")]
    headers.push("xcb_image");

    #[cfg(feature = "keysyms")]
    headers.push("xcb_keysyms");

    #[cfg(feature = "present")]
    headers.push("present");

    #[cfg(feature = "randr")]
    headers.push("randr");

    #[cfg(feature = "record")]
    headers.push("record");

    #[cfg(feature = "render")]
    headers.push("render");

    #[cfg(feature = "res")]
    headers.push("res");

    #[cfg(feature = "screensaver")]
    headers.push("screensaver");

    #[cfg(feature = "shape")]
    headers.push("shape");

    #[cfg(feature = "shm")]
    headers.push("shm");

    #[cfg(feature = "sync")]
    headers.push("sync");

    #[cfg(feature = "xc_misc")]
    headers.push("xc_misc");

    #[cfg(feature = "xevie")]
    headers.push("xevie");

    #[cfg(feature = "xf86dri")]
    headers.push("xf86dri");

    #[cfg(feature = "xfixes")]
    headers.push("xfixes");

    #[cfg(feature = "xinerama")]
    headers.push("xinerama");

    #[cfg(feature = "xinput")]
    headers.push("xinput");

    #[cfg(feature = "xkb")]
    headers.push("xkb");

    #[cfg(feature = "xprint")]
    headers.push("xprint");

    #[cfg(feature = "xselinux")]
    headers.push("xselinux");

    #[cfg(feature = "xtest")]
    headers.push("xtest");

    #[cfg(feature = "xv")]
    headers.push("xv");

    #[cfg(feature = "xvmc")]
    headers.push("xvmc");

    let mut wrapper = String::new();
    for header in headers {
        wrapper.push_str(&format!("#include <xcb/{}.h>\n", header));
    }

    let bindings = bindgen::Builder::default()
        .header_contents("wrapper.h", &wrapper)
        .whitelist_type("xcb_.*")
        .whitelist_function("xcb_.*")
        .whitelist_var("XCB_.*")
        .whitelist_var("X_.*")
        .prepend_enum_name(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
