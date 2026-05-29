#!/usr/bin/env bash
set -euo pipefail

# ─── colours ────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
ok()   { echo -e "${GREEN}✓${NC} $*"; }
info() { echo -e "${CYAN}▶${NC} $*"; }
warn() { echo -e "${YELLOW}⚠${NC} $*"; }
die()  { echo -e "${RED}✗ $*${NC}" >&2; exit 1; }

echo ""
echo -e "${CYAN}╔══════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     OmegaDeck — dev environment       ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════╝${NC}"
echo ""

# ─── root / sudo ─────────────────────────────────────────────────────────────
if [[ $EUID -eq 0 ]]; then
    die "Do not run this script as root. Run it as your normal user — it will ask for sudo when needed."
fi

info "Checking sudo access (system packages require root)..."
sudo -v || die "sudo access is required."
# Keep sudo alive for the duration of the script.
( while true; do sudo -n true; sleep 50; done ) &
SUDO_KEEPER=$!
trap 'kill $SUDO_KEEPER 2>/dev/null; true' EXIT

# ─── detect distro ───────────────────────────────────────────────────────────
if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    DISTRO="${ID:-unknown}"
    DISTRO_LIKE="${ID_LIKE:-}"
else
    DISTRO="unknown"
    DISTRO_LIKE=""
fi

is_debian() { [[ "$DISTRO" == "debian" || "$DISTRO" == "ubuntu" || "$DISTRO_LIKE" == *"debian"* ]]; }
is_arch()   { [[ "$DISTRO" == "arch"   || "$DISTRO" == "manjaro" || "$DISTRO_LIKE" == *"arch"* ]]; }
is_fedora() { [[ "$DISTRO" == "fedora" || "$DISTRO" == "rhel"    || "$DISTRO_LIKE" == *"fedora"* || "$DISTRO_LIKE" == *"rhel"* ]]; }

# ─── 1. System packages ──────────────────────────────────────────────────────
info "Installing system dependencies..."

if is_debian; then
    sudo apt-get update -qq
    # libayatana-appindicator3-dev is the modern replacement for libappindicator3-dev;
    # they conflict with each other so only one can be installed.
    sudo apt-get install -y \
        curl wget git build-essential pkg-config \
        libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
        patchelf libudev-dev xdg-utils libdbus-1-dev \
        libssl-dev libgtk-3-dev
    ok "APT packages installed."

    # Install Node.js LTS via NodeSource (the Ubuntu repo package has broken deps).
    if ! command -v node &>/dev/null; then
        info "Installing Node.js LTS via NodeSource..."
        curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
        sudo apt-get install -y nodejs
        ok "Node.js installed: $(node --version)"
    else
        ok "Node.js already installed: $(node --version)"
    fi
elif is_arch; then
    sudo pacman -Syu --noconfirm
    sudo pacman -S --noconfirm --needed \
        curl wget git base-devel pkg-config \
        webkit2gtk-4.1 libappindicator-gtk3 librsvg \
        patchelf libudev0-shim xdg-utils dbus \
        openssl gtk3 \
        nodejs npm
    ok "pacman packages installed."
elif is_fedora; then
    sudo dnf install -y \
        curl wget git gcc gcc-c++ make pkg-config \
        webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel \
        patchelf systemd-devel xdg-utils dbus-devel \
        openssl-devel gtk3-devel \
        nodejs npm
    ok "DNF packages installed."
else
    warn "Unrecognised distro '$DISTRO'. Skipping system packages — install them manually if the build fails."
    warn "Required: libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libudev-dev xdg-utils libdbus-1-dev"
fi

# ─── 2. Rust (via rustup) ────────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
    ok "Rust already installed: $(rustc --version)"
else
    info "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    ok "Rust installed."
fi

# Make sure ~/.cargo/bin is in PATH for the rest of this script.
export PATH="$HOME/.cargo/bin:$PATH"

# ─── 3. Deno ─────────────────────────────────────────────────────────────────
if command -v deno &>/dev/null; then
    ok "Deno already installed: $(deno --version | head -1)"
else
    info "Installing Deno..."
    curl -fsSL https://deno.land/install.sh | sh
    # Deno installs to ~/.deno/bin by default.
    export DENO_INSTALL="$HOME/.deno"
    export PATH="$DENO_INSTALL/bin:$PATH"
    ok "Deno installed."
fi

# ─── 4. Add paths to shell profile ───────────────────────────────────────────
PROFILE=""
if [[ -f "$HOME/.zshrc" ]];  then PROFILE="$HOME/.zshrc";
elif [[ -f "$HOME/.bashrc" ]]; then PROFILE="$HOME/.bashrc";
elif [[ -f "$HOME/.profile" ]]; then PROFILE="$HOME/.profile"; fi

if [[ -n "$PROFILE" ]]; then
    if ! grep -q '\.cargo/bin' "$PROFILE" 2>/dev/null; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$PROFILE"
        ok "Added ~/.cargo/bin to $PROFILE"
    fi
    if ! grep -q '\.deno/bin' "$PROFILE" 2>/dev/null; then
        echo 'export DENO_INSTALL="$HOME/.deno"' >> "$PROFILE"
        echo 'export PATH="$DENO_INSTALL/bin:$PATH"' >> "$PROFILE"
        ok "Added ~/.deno/bin to $PROFILE"
    fi
    # On Ubuntu systems with Snap, /snap/core*/lib gets injected into the
    # dynamic linker search path and its libpthread.so.0 is missing
    # __libc_pthread_init (GLIBC_PRIVATE), causing Tauri binaries to crash.
    # Prepending the system lib directory ensures the correct libpthread is used.
    if [[ -d /snap ]] && ! grep -q 'LD_LIBRARY_PATH.*usr/lib' "$PROFILE" 2>/dev/null; then
        echo 'export LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}' >> "$PROFILE"
        ok "Added LD_LIBRARY_PATH fix for Snap/GLIBC conflict to $PROFILE"
    fi
fi

# ─── 5. Frontend dependencies (npm + deno) ───────────────────────────────────
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_DIR"

info "Installing frontend npm dependencies..."
npm install
ok "npm dependencies installed."

info "Installing Deno dependencies..."
deno install
ok "Deno dependencies installed."

# ─── 6. Done ─────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}╔══════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   All dependencies installed!        ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════╝${NC}"
echo ""
echo "  To start OmegaDeck in development mode:"
echo ""
echo -e "    ${CYAN}cd $(pwd)${NC}"
echo -e "    ${CYAN}deno run -A npm:@tauri-apps/cli dev${NC}"
echo ""
echo "  Or to build a production binary:"
echo ""
echo -e "    ${CYAN}deno run -A npm:@tauri-apps/cli build${NC}"
echo ""
if [[ -n "$PROFILE" ]]; then
    echo -e "  ${YELLOW}Note:${NC} reload your shell or run  source $PROFILE  to pick up the new PATH entries."
    echo ""
fi
