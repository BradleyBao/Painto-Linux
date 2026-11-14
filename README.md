# Painto Linux

Rust/GTK4/Cairo Linux version of [Painto](https://github.com/BradleyBao/Painto).

## Production

Currently, Painto-Linux has achieved MVP (Monimum Viable Product). 

## Development

On Ubuntu, install the native development packages first:

```sh
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev libcairo2-dev pkg-config
pkg-config --modversion gtk4 cairo
```

Building also requires `glib-compile-resources`, which is provided with the GLib development tools. Toolbar SVG icons are embedded in the executable.

In a Plasma X11 session, run:

```sh
GDK_BACKEND=x11 cargo run
```

In a Wayland session with layer-shell support, run:

```sh
GDK_BACKEND=wayland cargo run
```

The toolbar switches between Computer (click-through), Draw, whole-stroke Erase, and Pixel erase. Draw supports pen color and width; Clear removes all annotations. The current canvas covers one monitor, and annotations are kept only in memory. Drag the Move handle to reposition the toolbar on either X11 or Wayland.

If you see many `undefined symbol: gtk_...` linker errors after a previous `DOCS_RS=1` build, run `cargo clean` once and then `GDK_BACKEND=x11 cargo run` again.
