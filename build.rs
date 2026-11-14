use std::{env, path::PathBuf, process::Command};

fn main() {
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set")).join("icons.gresource");
    let files = [
        "draggable-symbolic.svg",
        "pointer-left-symbolic.svg",
        "pen-symbolic.svg",
        "eraser-symbolic.svg",
        "tool-eraser-symbolic.svg",
        "draw-eraser-delete-objects-symbolic.svg",
        "gtk-close-symbolic.svg",
    ];

    println!("cargo:rerun-if-changed=resources/icons.gresource.xml");
    for file in files {
        println!("cargo:rerun-if-changed=resources/{file}");
    }

    let status = Command::new("glib-compile-resources")
        .args(["--sourcedir=resources", "--target"])
        .arg(&output)
        .arg("resources/icons.gresource.xml")
        .status()
        .expect("glib-compile-resources is required to build Painto icons");
    assert!(status.success(), "glib-compile-resources failed");
}
