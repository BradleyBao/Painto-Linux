# Painto Linux

Rust/GTK4/Cairo starter for an X11 version of [Painto](https://github.com/BradleyBao/Painto).

## Development

On Ubuntu, install the native development packages first:

```sh
sudo apt install libgtk-4-dev libcairo2-dev pkg-config
pkg-config --modversion gtk4 cairo
```

In a Plasma X11 session, run:

```sh
GDK_BACKEND=x11 cargo run
```

If you see many `undefined symbol: gtk_...` linker errors after a previous `DOCS_RS=1` build, run `cargo clean` once and then `GDK_BACKEND=x11 cargo run` again.

This project is currently only supporting x11, wayland will be supported in the future. 
