use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=xcb");

    let mut headers = Vec::new();
    headers.push("xcb");

    #[cfg(feature = "bigreq")]
    headers.push("bigreq");

    #[cfg(feature = "composite")]
    {
        headers.push("composite");
        println!("cargo:rustc-link-lib=xcb-composite");
    }

    #[cfg(feature = "cursor")]
    {
        headers.push("xcb_cursor");
        println!("cargo:rustc-link-lib=xcb-cursor");
    }

    #[cfg(feature = "damage")]
    {
        headers.push("damage");
        println!("cargo:rustc-link-lib=xcb-damage");
    }

    #[cfg(feature = "dpms")]
    {
        headers.push("dpms");
        println!("cargo:rustc-link-lib=xcb-dpms");
    }

    #[cfg(feature = "dri2")]
    {
        headers.push("dri2");
        println!("cargo:rustc-link-lib=xcb-dri2");
    }

    #[cfg(feature = "dri3")]
    {
        headers.push("dri3");
        println!("cargo:rustc-link-lib=xcb-dri3");
    }

    #[cfg(feature = "ewmh")]
    {
        headers.push("xcb_ewmh");
        println!("cargo:rustc-link-lib=xcb-ewmh");
    }

    #[cfg(feature = "ge")]
    {
        headers.push("ge");
        println!("cargo:rustc-link-lib=xcb-ge");
    }

    #[cfg(feature = "glx")]
    {
        headers.push("glx");
        println!("cargo:rustc-link-lib=xcb-glx");
    }

    #[cfg(feature = "icccm")]
    {
        headers.push("xcb_icccm");
        println!("cargo:rustc-link-lib=xcb-icccm");
    }

    #[cfg(feature = "image")]
    {
        headers.push("xcb_image");
        println!("cargo:rustc-link-lib=xcb-image");
    }

    #[cfg(feature = "keysyms")]
    {
        headers.push("xcb_keysyms");
        println!("cargo:rustc-link-lib=xcb-keysyms");
    }

    #[cfg(feature = "present")]
    {
        headers.push("present");
        println!("cargo:rustc-link-lib=xcb-present");
    }

    #[cfg(feature = "randr")]
    {
        headers.push("randr");
        println!("cargo:rustc-link-lib=xcb-randr");
    }

    #[cfg(feature = "record")]
    {
        headers.push("record");
        println!("cargo:rustc-link-lib=xcb-record");
    }

    #[cfg(feature = "render")]
    {
        headers.push("render");
        println!("cargo:rustc-link-lib=xcb-render");
    }

    #[cfg(feature = "res")]
    {
        headers.push("res");
        println!("cargo:rustc-link-lib=xcb-res");
    }

    #[cfg(feature = "screensaver")]
    {
        headers.push("screensaver");
        println!("cargo:rustc-link-lib=xcb-screensaver");
    }

    #[cfg(feature = "shape")]
    {
        headers.push("shape");
        println!("cargo:rustc-link-lib=xcb-shape");
    }

    #[cfg(feature = "shm")]
    {
        headers.push("shm");
        println!("cargo:rustc-link-lib=xcb-shm");
    }

    #[cfg(feature = "sync")]
    {
        headers.push("sync");
        println!("cargo:rustc-link-lib=xcb-sync");
    }

    #[cfg(feature = "xc_misc")]
    headers.push("xc_misc");

    #[cfg(feature = "xf86dri")]
    {
        headers.push("xf86dri");
        println!("cargo:rustc-link-lib=xcb-xf86dri");
    }

    #[cfg(feature = "xfixes")]
    {
        headers.push("xfixes");
        println!("cargo:rustc-link-lib=xcb-xfixes");
    }

    #[cfg(feature = "xinerama")]
    {
        headers.push("xinerama");
        println!("cargo:rustc-link-lib=xcb-xinerama");
    }

    #[cfg(feature = "xinput")]
    {
        headers.push("xinput");
        println!("cargo:rustc-link-lib=xcb-xinput");
    }

    #[cfg(feature = "xkb")]
    {
        headers.push("xkb");
        println!("cargo:rustc-link-lib=xcb-xkb");
    }

    #[cfg(feature = "xprint")]
    headers.push("xprint");

    #[cfg(feature = "xselinux")]
    {
        headers.push("xselinux");
        println!("cargo:rustc-link-lib=xcb-xselinux");
    }

    #[cfg(feature = "xtest")]
    {
        headers.push("xtest");
        println!("cargo:rustc-link-lib=xcb-xtest");
    }

    #[cfg(feature = "xv")]
    {
        headers.push("xv");
        println!("cargo:rustc-link-lib=xcb-xv");
    }

    #[cfg(feature = "xvmc")]
    {
        headers.push("xvmc");
        println!("cargo:rustc-link-lib=xcb-xvmc");
    }

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
