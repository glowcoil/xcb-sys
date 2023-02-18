use std::env;
use std::path::PathBuf;

mod gen;

fn main() {
    let mut sources = Vec::new();

    sources.push("xproto");

    #[cfg(feature = "bigreq")]
    sources.push("bigreq");
    #[cfg(feature = "composite")]
    sources.push("composite");
    #[cfg(feature = "damage")]
    sources.push("damage");
    #[cfg(feature = "dpms")]
    sources.push("dpms");
    #[cfg(feature = "dri2")]
    sources.push("dri2");
    #[cfg(feature = "dri3")]
    sources.push("dri3");
    #[cfg(feature = "ge")]
    sources.push("ge");
    #[cfg(feature = "glx")]
    sources.push("glx");
    #[cfg(feature = "present")]
    sources.push("present");
    #[cfg(feature = "randr")]
    sources.push("randr");
    #[cfg(feature = "record")]
    sources.push("record");
    #[cfg(feature = "render")]
    sources.push("render");
    #[cfg(feature = "res")]
    sources.push("res");
    #[cfg(feature = "screensaver")]
    sources.push("screensaver");
    #[cfg(feature = "shape")]
    sources.push("shape");
    #[cfg(feature = "shm")]
    sources.push("shm");
    #[cfg(feature = "sync")]
    sources.push("sync");
    #[cfg(feature = "xc_misc")]
    sources.push("xc_misc");
    #[cfg(feature = "xevie")]
    sources.push("xevie");
    #[cfg(feature = "xf86dri")]
    sources.push("xf86dri");
    #[cfg(feature = "xfixes")]
    sources.push("xfixes");
    #[cfg(feature = "xinerama")]
    sources.push("xinerama");
    #[cfg(feature = "xinput")]
    sources.push("xinput");
    #[cfg(feature = "xkb")]
    sources.push("xkb");
    #[cfg(feature = "xprint")]
    sources.push("xprint");
    #[cfg(feature = "xselinux")]
    sources.push("xselinux");
    #[cfg(feature = "xtest")]
    sources.push("xtest");
    #[cfg(feature = "xv")]
    sources.push("xv");
    #[cfg(feature = "xvmc")]
    sources.push("xvmc");

    let mut out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    out_path.push("bindings.rs");
    gen::gen(&sources[..], &out_path);

    println!("cargo:rustc-link-lib=xcb");

    #[cfg(feature = "composite")]
    println!("cargo:rustc-link-lib=xcb-composite");
    #[cfg(feature = "damage")]
    println!("cargo:rustc-link-lib=xcb-damage");
    #[cfg(feature = "dpms")]
    println!("cargo:rustc-link-lib=xcb-dpms");
    #[cfg(feature = "dri2")]
    println!("cargo:rustc-link-lib=xcb-dri2");
    #[cfg(feature = "dri3")]
    println!("cargo:rustc-link-lib=xcb-dri3");
    #[cfg(feature = "ge")]
    println!("cargo:rustc-link-lib=xcb-ge");
    #[cfg(feature = "glx")]
    println!("cargo:rustc-link-lib=xcb-glx");
    #[cfg(feature = "present")]
    println!("cargo:rustc-link-lib=xcb-present");
    #[cfg(feature = "randr")]
    println!("cargo:rustc-link-lib=xcb-randr");
    #[cfg(feature = "record")]
    println!("cargo:rustc-link-lib=xcb-record");
    #[cfg(feature = "render")]
    println!("cargo:rustc-link-lib=xcb-render");
    #[cfg(feature = "res")]
    println!("cargo:rustc-link-lib=xcb-res");
    #[cfg(feature = "screensaver")]
    println!("cargo:rustc-link-lib=xcb-screensaver");
    #[cfg(feature = "shape")]
    println!("cargo:rustc-link-lib=xcb-shape");
    #[cfg(feature = "shm")]
    println!("cargo:rustc-link-lib=xcb-shm");
    #[cfg(feature = "sync")]
    println!("cargo:rustc-link-lib=xcb-sync");
    #[cfg(feature = "xevie")]
    println!("cargo:rustc-link-lib=xcb-xevie");
    #[cfg(feature = "xf86dri")]
    println!("cargo:rustc-link-lib=xcb-xf86dri");
    #[cfg(feature = "xfixes")]
    println!("cargo:rustc-link-lib=xcb-xfixes");
    #[cfg(feature = "xinerama")]
    println!("cargo:rustc-link-lib=xcb-xinerama");
    #[cfg(feature = "xinput")]
    println!("cargo:rustc-link-lib=xcb-xinput");
    #[cfg(feature = "xkb")]
    println!("cargo:rustc-link-lib=xcb-xkb");
    #[cfg(feature = "xprint")]
    println!("cargo:rustc-link-lib=xcb-xprint");
    #[cfg(feature = "xselinux")]
    println!("cargo:rustc-link-lib=xcb-xselinux");
    #[cfg(feature = "xtest")]
    println!("cargo:rustc-link-lib=xcb-xtest");
    #[cfg(feature = "xv")]
    println!("cargo:rustc-link-lib=xcb-xv");
    #[cfg(feature = "xvmc")]
    println!("cargo:rustc-link-lib=xcb-xvmc");
}
