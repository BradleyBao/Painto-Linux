# Flatpak preparation

These are editable templates, not a submitted Flathub package. Keep the application ID `com.tianyibrad.Painto` consistent in the manifest, desktop file, metainfo, app icon filename, and `src/main.rs`.

## Before building

1. The repository has an MIT `LICENSE`, matching the upstream Painto project. Keep upstream copyright notices for any reused work. Complete `ASSETS.md` for the bundled SVGs.
2. Confirm the application icon and text in `data/`. Check the icon at 128 px and smaller sizes. Add real screenshots to the metainfo XML before submitting to Flathub. Update the release version and date as needed.
3. Install `flatpak-builder`, the GNOME Platform and SDK matching `runtime-version`, and the Rust SDK extension. This manifest uses GNOME 49 as an editable starting point.
4. Generate `flatpak/cargo-sources.json` from `Cargo.lock` with the official `flatpak-cargo-generator.py` from `flatpak-builder-tools`:

   ```sh
   python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
   ```

5. Commit and push these files before testing the manifest's `main` source. For a Flathub submission, replace `branch: main` with an immutable release tag and commit hash, and regenerate the Cargo sources whenever `Cargo.lock` changes.

## Local verification

Build and install the template with `scripts/build_flatpak.sh`. The script ensures a user-level Flathub remote exists before installing the GNOME SDK and Rust extension:

```sh
./scripts/build_flatpak.sh
flatpak run --user com.tianyibrad.Painto
appstreamcli validate data/com.tianyibrad.Painto.metainfo.xml
desktop-file-validate data/com.tianyibrad.Painto.desktop
```

If another machine reports `No remote refs found for flathub`, check its user remote and SDK reference with:

```sh
flatpak remotes --user --show-details
flatpak remote-info --user flathub org.gnome.Sdk//49
```

The `XDG_DATA_DIRS` notice only affects whether the desktop launcher appears immediately. Log out and back in after installing Flatpak if the application is missing from the menu.

Check both X11 and Wayland sessions using `TESTING.md`. On Wayland, test a compositor with layer-shell support. The canvas currently covers one monitor and drawings are kept only in memory. This manifest exposes both X11 and Wayland sockets so users can choose the X11 backend when a Wayland compositor lacks layer-shell support.
