#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v flatpak-builder >/dev/null 2>&1; then
    if [[ -f /etc/debian_version ]]; then
        sudo apt-get update
        sudo apt-get install -y flatpak-builder
    elif [[ -f /etc/fedora-release ]]; then
        sudo dnf install -y flatpak-builder
    elif [[ -f /etc/arch-release ]]; then
        sudo pacman -S --needed flatpak-builder
    else
        printf 'Install flatpak-builder with your distribution package manager.\n' >&2
        exit 1
    fi
fi

flatpak remote-add --user --if-not-exists flathub \
    https://flathub.org/repo/flathub.flatpakrepo

if ! flatpak remote-info --user flathub org.gnome.Sdk//49 >/dev/null; then
    printf 'GNOME SDK 49 is not available from the user Flathub remote.\n' >&2
    printf 'Check: flatpak remotes --user --show-details\n' >&2
    exit 1
fi

flatpak-builder --user --install-deps-from=flathub --install \
    --force-clean flatpak-build flatpak/com.tianyibrad.Painto.yml

flatpak info --user com.tianyibrad.Painto
printf '\nBuild installed. Run it with: flatpak run --user com.tianyibrad.Painto\n'
