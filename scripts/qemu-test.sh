#!/bin/bash
# QEMU test script for CodeVerse Compositor
# Tests the compositor in a real Linux environment with Wayland support

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
QEMU_DIR="$PROJECT_ROOT/qemu-test"
COMPOSITOR_BINARY="$PROJECT_ROOT/target/release/codeverse-compositor"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   CodeVerse Compositor - QEMU Test Script     ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════╝${NC}"
echo ""

# Check if compositor is built
if [ ! -f "$COMPOSITOR_BINARY" ]; then
    echo -e "${YELLOW}Compositor not found at $COMPOSITOR_BINARY${NC}"
    echo -e "${YELLOW}Building compositor in release mode...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
    echo -e "${GREEN}✓ Compositor built successfully${NC}"
fi

# Create QEMU test directory
mkdir -p "$QEMU_DIR"
cd "$QEMU_DIR"

# Fedora ISO details (Fedora 43 Workstation - excellent Wayland support)
FEDORA_VERSION="43"
FEDORA_ISO="Fedora-Workstation-Live-${FEDORA_VERSION}-1.6.x86_64.iso"
FEDORA_URL="https://download.fedoraproject.org/pub/fedora/linux/releases/${FEDORA_VERSION}/Workstation/x86_64/iso/${FEDORA_ISO}"
# Alternative mirrors (in case primary fails)
FEDORA_MIRROR1="https://mirror.arizona.edu/fedora/linux/releases/${FEDORA_VERSION}/Workstation/x86_64/iso/${FEDORA_ISO}"
FEDORA_MIRROR2="https://mirrors.rit.edu/fedora/fedora/linux/releases/${FEDORA_VERSION}/Workstation/x86_64/iso/${FEDORA_ISO}"
FEDORA_MIRROR3="https://ftp.lysator.liu.se/pub/fedora/linux/releases/${FEDORA_VERSION}/Workstation/x86_64/iso/${FEDORA_ISO}"

# Alternative: Arch Linux ISO (lighter, rolling release)
ARCH_ISO="archlinux-x86_64.iso"
ARCH_URL="https://mirrors.kernel.org/archlinux/iso/latest/${ARCH_ISO}"

echo -e "${BLUE}Choose Linux distribution for testing:${NC}"
echo "1) Fedora Workstation 43 (recommended - native Wayland)"
echo "2) Arch Linux (lightweight, rolling release)"
echo ""
read -p "Enter choice [1-2] (default: 1): " DISTRO_CHOICE
DISTRO_CHOICE=${DISTRO_CHOICE:-1}

if [ "$DISTRO_CHOICE" = "2" ]; then
    ISO_FILE="$ARCH_ISO"
    ISO_URL="$ARCH_URL"
    DISTRO_NAME="Arch Linux"
else
    ISO_FILE="$FEDORA_ISO"
    ISO_URL="$FEDORA_URL"
    DISTRO_NAME="Fedora Workstation"
fi

# Download ISO if not present
if [ ! -f "$ISO_FILE" ]; then
    echo -e "${YELLOW}Downloading $DISTRO_NAME ISO...${NC}"
    echo -e "${BLUE}URL: $ISO_URL${NC}"
    echo -e "${YELLOW}This may take a while (1.5-2GB download)...${NC}"

    # Try with wget first, fall back to curl
    DOWNLOAD_SUCCESS=false

    if command -v wget &> /dev/null; then
        # Try primary URL
        if wget -c "$ISO_URL" -O "$ISO_FILE"; then
            DOWNLOAD_SUCCESS=true
        elif [ "$DISTRO_CHOICE" = "1" ]; then
            # Try Fedora mirrors one by one
            echo -e "${YELLOW}Primary mirror failed. Trying alternative mirrors...${NC}"
            for mirror in "$FEDORA_MIRROR1" "$FEDORA_MIRROR2" "$FEDORA_MIRROR3"; do
                echo -e "${BLUE}Trying: $mirror${NC}"
                if wget -c "$mirror" -O "$ISO_FILE"; then
                    DOWNLOAD_SUCCESS=true
                    break
                fi
            done
        fi
    elif command -v curl &> /dev/null; then
        # Try primary URL
        if curl -L -C - "$ISO_URL" -o "$ISO_FILE"; then
            DOWNLOAD_SUCCESS=true
        elif [ "$DISTRO_CHOICE" = "1" ]; then
            # Try Fedora mirrors one by one
            echo -e "${YELLOW}Primary mirror failed. Trying alternative mirrors...${NC}"
            for mirror in "$FEDORA_MIRROR1" "$FEDORA_MIRROR2" "$FEDORA_MIRROR3"; do
                echo -e "${BLUE}Trying: $mirror${NC}"
                if curl -L -C - "$mirror" -o "$ISO_FILE"; then
                    DOWNLOAD_SUCCESS=true
                    break
                fi
            done
        fi
    else
        echo -e "${RED}Error: Neither wget nor curl found. Please install one of them.${NC}"
        exit 1
    fi

    if [ "$DOWNLOAD_SUCCESS" = false ]; then
        echo -e "${RED}Error: All download attempts failed.${NC}"
        echo -e "${YELLOW}You can manually download the ISO and place it in: $QEMU_DIR${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ ISO downloaded successfully${NC}"
else
    echo -e "${GREEN}✓ ISO already downloaded: $ISO_FILE${NC}"
fi

# Create a virtual disk for persistence (optional)
DISK_IMAGE="compositor-test.qcow2"
if [ ! -f "$DISK_IMAGE" ]; then
    echo -e "${YELLOW}Creating virtual disk (20GB)...${NC}"
    qemu-img create -f qcow2 "$DISK_IMAGE" 20G
    echo -e "${GREEN}✓ Virtual disk created${NC}"
fi

# Check for KVM support
KVM_OPTS=""
if [ -r /dev/kvm ]; then
    echo -e "${GREEN}✓ KVM acceleration available${NC}"
    KVM_OPTS="-enable-kvm -cpu host"
else
    echo -e "${YELLOW}⚠ KVM not available, using slower emulation${NC}"
fi

# Copy compositor binary to a shared location
mkdir -p "$QEMU_DIR/shared"
cp "$COMPOSITOR_BINARY" "$QEMU_DIR/shared/"
echo -e "${GREEN}✓ Compositor binary copied to shared directory${NC}"

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          Starting QEMU Virtual Machine         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}QEMU VM Configuration:${NC}"
echo "  • RAM: 4GB"
echo "  • CPUs: 4 cores"
echo "  • Graphics: virtio-vga-gl (OpenGL support)"
echo "  • Display: GTK with OpenGL"
echo "  • ISO: $ISO_FILE"
echo "  • Shared folder: $QEMU_DIR/shared"
echo ""
echo -e "${YELLOW}Choose testing mode:${NC}"
echo "  ${GREEN}A)${NC} Test Winit Backend (nested in GNOME Wayland)"
echo "  ${GREEN}B)${NC} Test DRM Backend (direct on TTY - production mode)"
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Option A: Testing Winit Backend (Nested Session)${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}1. Boot into the live system${NC}"
echo "   - Select 'Try Fedora' or boot Arch live environment"
echo ""
echo -e "${BLUE}2. Open a terminal (Ctrl+Alt+T or search for 'Terminal')${NC}"
echo ""
echo -e "${BLUE}3. Install dependencies:${NC}"
echo "   ${GREEN}# For Fedora:${NC}"
echo "   sudo dnf install -y weston alacritty foot"
echo ""
echo "   ${GREEN}# For Arch:${NC}"
echo "   sudo pacman -Sy --noconfirm weston alacritty foot"
echo ""
echo -e "${BLUE}4. Mount shared folder (if needed):${NC}"
echo "   # The binary should be accessible in the live environment"
echo "   # Or use SSH: ${GREEN}ssh -p 2222 liveuser@localhost${NC}"
echo ""
echo -e "${BLUE}5. Run the compositor (will use Winit backend automatically):${NC}"
echo "   ${GREEN}RUST_LOG=debug ./codeverse-compositor${NC}"
echo "   # It detects WAYLAND_DISPLAY and uses nested winit backend"
echo ""
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Option B: Testing DRM Backend (TTY - Production Mode)${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}1. Boot into the live system${NC}"
echo "   - Select 'Try Fedora' or boot Arch live environment"
echo ""
echo -e "${BLUE}2. Switch to a TTY (Ctrl+Alt+F3)${NC}"
echo "   - Login as 'liveuser' (usually no password)"
echo ""
echo -e "${BLUE}3. Stop the display server:${NC}"
echo "   ${GREEN}# For Fedora:${NC}"
echo "   sudo systemctl stop gdm"
echo ""
echo "   ${GREEN}# For Arch:${NC}"
echo "   sudo systemctl stop lightdm  # or whatever DM is running"
echo ""
echo -e "${BLUE}4. Verify you're on a clean TTY:${NC}"
echo "   ${GREEN}echo \$DISPLAY${NC}         # Should be empty"
echo "   ${GREEN}echo \$WAYLAND_DISPLAY${NC} # Should be empty"
echo "   ${GREEN}ls /dev/dri/${NC}          # Should show card0, renderD128, etc."
echo ""
echo -e "${BLUE}5. Set up permissions:${NC}"
echo "   ${GREEN}sudo usermod -aG video,input \$USER${NC}"
echo "   ${GREEN}newgrp video${NC}  # Or log out and back in"
echo ""
echo -e "${BLUE}6. Copy the compositor binary:${NC}"
echo "   # SSH from host machine:"
echo "   ${GREEN}scp -P 2222 $QEMU_DIR/shared/codeverse-compositor liveuser@localhost:/tmp/${NC}"
echo ""
echo -e "${BLUE}7. Run the compositor (will use DRM backend automatically):${NC}"
echo "   ${GREEN}cd /tmp${NC}"
echo "   ${GREEN}chmod +x codeverse-compositor${NC}"
echo "   ${GREEN}RUST_LOG=debug ./codeverse-compositor${NC}"
echo ""
echo -e "${BLUE}8. Expected output:${NC}"
echo "   ${GREEN}INFO  CodeVerse Compositor starting...${NC}"
echo "   ${GREEN}INFO  Using DRM backend (TTY)${NC}"
echo "   ${GREEN}INFO  Session created on seat: seat0${NC}"
echo "   ${GREEN}INFO  Using renderD128 as primary GPU${NC}"
echo "   ${GREEN}INFO  Connector Virtual-1 connected on crtc ...${NC}"
echo "   ${GREEN}INFO  Wayland socket: wayland-0${NC}"
echo ""
echo -e "${BLUE}9. Test with Wayland clients (from another TTY or SSH):${NC}"
echo "   # Switch to TTY4 (Ctrl+Alt+F4) or SSH in"
echo "   ${GREEN}export WAYLAND_DISPLAY=wayland-0${NC}"
echo "   ${GREEN}export XDG_RUNTIME_DIR=/run/user/\$(id -u)${NC}"
echo "   ${GREEN}weston-terminal${NC}  # or foot, or alacritty"
echo ""
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}Keyboard shortcuts (both backends):${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo "   • Press ${GREEN}F12${NC} to spawn a terminal (testing only)"
echo "   • Press ${GREEN}Super+d${NC} to open launcher"
echo "   • Press ${GREEN}Super+h/j/k/l${NC} to navigate windows"
echo "   • Press ${GREEN}Super+b/v${NC} to split windows"
echo "   • Press ${GREEN}Super+1-9${NC} to switch workspaces"
echo "   • Press ${GREEN}Super+e/w/s/t${NC} to change layouts"
echo "   • Press ${GREEN}Super+Shift+Space${NC} to toggle floating"
echo "   • Press ${GREEN}Super+Shift+Q${NC} to quit compositor"
echo ""
echo -e "${YELLOW}Press Enter to start QEMU (or Ctrl+C to cancel)...${NC}"
read

# Start QEMU
qemu-system-x86_64 \
    $KVM_OPTS \
    -m 4G \
    -smp 4 \
    -cdrom "$ISO_FILE" \
    -boot d \
    -hda "$DISK_IMAGE" \
    -device virtio-vga-gl \
    -display gtk,gl=on \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -usb \
    -device usb-tablet \
    -name "CodeVerse Compositor Test VM" \
    "$@"

echo ""
echo -e "${GREEN}QEMU session ended${NC}"
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}Troubleshooting Tips:${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}Permission denied on /dev/dri:${NC}"
echo "  sudo usermod -aG video,input \$USER && newgrp video"
echo ""
echo -e "${GREEN}\"Failed to create session\" error:${NC}"
echo "  sudo chmod 666 /dev/dri/*  # Quick fix for testing"
echo ""
echo -e "${GREEN}No GPU found:${NC}"
echo "  ls -la /dev/dri/  # Should see card0, renderD128"
echo "  lsmod | grep virtio  # Check virtio-gpu is loaded"
echo ""
echo -e "${GREEN}Compositor crashes immediately:${NC}"
echo "  RUST_LOG=trace ./codeverse-compositor  # More verbose output"
echo "  dmesg | tail -50  # Check kernel messages"
echo ""
echo -e "${GREEN}Can't connect Wayland clients:${NC}"
echo "  ls -la \$XDG_RUNTIME_DIR/wayland-0  # Should exist"
echo "  export WAYLAND_DISPLAY=wayland-0"
echo "  export XDG_RUNTIME_DIR=/run/user/\$(id -u)"
echo ""
echo -e "${GREEN}Want to switch between backends manually:${NC}"
echo "  # Force DRM backend: unset DISPLAY WAYLAND_DISPLAY"
echo "  # Force Winit backend: export DISPLAY=:0 (or WAYLAND_DISPLAY=wayland-0)"
echo ""
