#!/bin/bash

# Check if flatpak-builder is installed
if ! command -v flatpak-builder &> /dev/null; then
    # Install flatpak-builder if not found
    # Check if the system is Debian-based
    if [ -f /etc/debian_version ]; then
        sudo apt update
        sudo apt install -y flatpak-builder
    # Check if the system is Fedora-based
    elif [ -f /etc/fedora-release ]; then
        sudo dnf install -y flatpak-builder
    elif [ -f /etc/arch-release ]; then
        sudo pacman -S flatpak-builder
    else
        echo "Unsupported Linux distribution. Please install flatpak-builder manually."
        exit 1
    fi

    flatpak-builder --user --install-deps-from=flathub --install \
        --force-clean flatpak-build flatpak/com.tianyibrad.Painto.yml
fi

flatpak run com.tianyibrad.Painto
flatpak info com.tianyibrad.Painto