#!/bin/bash
# Helper script to deploy compositor to running QEMU VM
# Run this from your host machine while the VM is running

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_ROOT/target/release/codeverse-compositor"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Deploy Compositor to QEMU VM${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Compositor not built${NC}"
    echo -e "${YELLOW}Run: cargo build --release${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Found compositor binary${NC}"

# Build if source is newer
if [ "$PROJECT_ROOT/Cargo.toml" -nt "$BINARY" ]; then
    echo -e "${YELLOW}Source code changed, rebuilding...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
    echo -e "${GREEN}✓ Rebuilt successfully${NC}"
fi

# VM connection details
VM_HOST="localhost"
VM_PORT="2222"
VM_USER="liveuser"
VM_DEST="/tmp/codeverse-compositor"

echo ""
echo -e "${BLUE}Deploying to VM...${NC}"
echo "  Host: $VM_HOST:$VM_PORT"
echo "  User: $VM_USER"
echo "  Dest: $VM_DEST"
echo ""

# Try to copy with SCP
echo -e "${YELLOW}Copying binary (password is usually empty, just press Enter)...${NC}"

if scp -P "$VM_PORT" "$BINARY" "$VM_USER@$VM_HOST:$VM_DEST" 2>/dev/null; then
    echo -e "${GREEN}✓ Binary copied successfully${NC}"

    # Make it executable
    if ssh -p "$VM_PORT" "$VM_USER@$VM_HOST" "chmod +x $VM_DEST" 2>/dev/null; then
        echo -e "${GREEN}✓ Made executable${NC}"
    fi

    echo ""
    echo -e "${GREEN}Deployment complete!${NC}"
    echo ""
    echo -e "${BLUE}To run in the VM:${NC}"
    echo ""
    echo -e "${YELLOW}For Winit backend (nested):${NC}"
    echo "  ${GREEN}RUST_LOG=debug $VM_DEST${NC}"
    echo ""
    echo -e "${YELLOW}For DRM backend (TTY):${NC}"
    echo "  1. Switch to TTY3: ${GREEN}Ctrl+Alt+F3${NC}"
    echo "  2. Stop display manager: ${GREEN}sudo systemctl stop gdm${NC}"
    echo "  3. Run: ${GREEN}RUST_LOG=debug $VM_DEST${NC}"
    echo ""

else
    echo -e "${RED}Failed to copy binary${NC}"
    echo ""
    echo -e "${YELLOW}Possible reasons:${NC}"
    echo "  • VM is not running (start it with: ./scripts/qemu-test.sh)"
    echo "  • SSH not configured in the live system"
    echo "  • Port 2222 is blocked by firewall"
    echo ""
    echo -e "${YELLOW}Alternative: Copy manually${NC}"
    echo "  1. In VM, open terminal"
    echo "  2. Copy binary from shared folder or rebuild from source"
    echo ""
fi
