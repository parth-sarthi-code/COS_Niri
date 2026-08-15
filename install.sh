#!/usr/bin/env bash
set -e

# ==============================================================================
# ChromeOS-Style Niri Bar (cos-niri-bar) Installer
# ==============================================================================

REPO="parth-sarthi-code/COS_Niri"
INSTALL_BIN_DIR="$HOME/.local/bin"
INSTALL_FONT_DIR="$HOME/.local/share/fonts/cos-niri"
CONFIG_DIR="$HOME/.config/cos-niri"
NIRI_DIR="$HOME/.config/niri"
GITHUB_RAW="https://raw.githubusercontent.com/${REPO}/main"

# ANSI Colors
BOLD="\033[1m"
GREEN="\033[0;32m"
BLUE="\033[0;34m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
NC="\033[0m"

log_info()    { echo -e "${BLUE}${BOLD}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}${BOLD}[OK]${NC} $1"; }
log_warn()    { echo -e "${YELLOW}${BOLD}[WARN]${NC} $1"; }
log_error()   { echo -e "${RED}${BOLD}[ERROR]${NC} $1"; }

# Detect if running from within local repository clone
IS_LOCAL_REPO=0
REPO_DIR=""
if [ -n "${BASH_SOURCE[0]}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    POTENTIAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [ -f "${POTENTIAL_DIR}/Cargo.toml" ] && [ -d "${POTENTIAL_DIR}/.config/niri" ]; then
        IS_LOCAL_REPO=1
        REPO_DIR="${POTENTIAL_DIR}"
    fi
fi

echo -e "${BOLD}Installing cos-niri-bar...${NC}\n"

# ------------------------------------------------------------------------------
# 1. Check & Install System Dependencies
# ------------------------------------------------------------------------------
log_info "Checking system dependencies..."

if command -v pacman >/dev/null 2>&1; then
    MISSING_PKGS=()
    PKGS=("gtk4" "gtk4-layer-shell" "fontconfig" "adwaita-icon-theme" "hicolor-icon-theme" "fuzzel" "swaybg")

    for pkg in "${PKGS[@]}"; do
        if ! pacman -Q "$pkg" >/dev/null 2>&1; then
            MISSING_PKGS+=("$pkg")
        fi
    done

    if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
        log_warn "Missing required packages: ${MISSING_PKGS[*]}"
        echo -e "Installing required packages via pacman..."
        sudo pacman -S --needed --noconfirm "${MISSING_PKGS[@]}"
    else
        log_success "All system dependencies are already installed."
    fi
elif command -v dnf >/dev/null 2>&1; then
    log_info "Fedora system detected. Ensure 'gtk4-devel', 'gtk4-layer-shell-devel', 'fuzzel', and 'swaybg' are installed."
elif command -v apt-get >/dev/null 2>&1; then
    log_info "Debian/Ubuntu detected. Ensure 'libgtk-4-dev', 'libgtk4-layer-shell-dev', 'fuzzel', and 'swaybg' are installed."
fi

# Check & Install Matugen (Material You dynamic theming engine)
if command -v matugen >/dev/null 2>&1; then
    log_success "matugen is already installed ($(matugen --version 2>/dev/null || echo 'installed'))."
else
    log_info "matugen not found. Installing matugen..."
    MATUGEN_INSTALLED=0
    if command -v pacman >/dev/null 2>&1 && pacman -Si matugen >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm matugen && MATUGEN_INSTALLED=1
    elif command -v yay >/dev/null 2>&1; then
        yay -S --needed --noconfirm matugen-bin 2>/dev/null && MATUGEN_INSTALLED=1
    elif command -v paru >/dev/null 2>&1; then
        paru -S --needed --noconfirm matugen-bin 2>/dev/null && MATUGEN_INSTALLED=1
    fi

    if [ "$MATUGEN_INSTALLED" -eq 0 ] && command -v cargo >/dev/null 2>&1; then
        log_info "Installing matugen via cargo..."
        cargo install matugen && MATUGEN_INSTALLED=1
    fi

    if [ "$MATUGEN_INSTALLED" -eq 0 ]; then
        log_info "Downloading pre-compiled matugen binary from GitHub..."
        mkdir -p "${INSTALL_BIN_DIR}"
        MATUGEN_TMP=$(mktemp -d)
        MATUGEN_URL="https://github.com/InioX/matugen/releases/latest/download/matugen-x86_64-unknown-linux-gnu.tar.gz"
        if curl -sSL -o "${MATUGEN_TMP}/matugen.tar.gz" "$MATUGEN_URL" 2>/dev/null; then
            tar -xzf "${MATUGEN_TMP}/matugen.tar.gz" -C "${INSTALL_BIN_DIR}" matugen 2>/dev/null || tar -xzf "${MATUGEN_TMP}/matugen.tar.gz" -C "${INSTALL_BIN_DIR}"
            chmod +x "${INSTALL_BIN_DIR}/matugen" 2>/dev/null || true
            if [ -f "${INSTALL_BIN_DIR}/matugen" ]; then
                MATUGEN_INSTALLED=1
                log_success "matugen binary installed to ${INSTALL_BIN_DIR}/matugen"
            fi
        fi
        rm -rf "${MATUGEN_TMP}"
    fi

    if [ "$MATUGEN_INSTALLED" -eq 1 ]; then
        log_success "matugen setup complete."
    else
        log_warn "Could not auto-install matugen. You can install it manually: cargo install matugen"
    fi
fi

# ------------------------------------------------------------------------------
# 2. Download Release Binary
# ------------------------------------------------------------------------------
log_info "Setting up binary directory at ${INSTALL_BIN_DIR}..."
mkdir -p "${INSTALL_BIN_DIR}"

TARGET_BIN="${INSTALL_BIN_DIR}/cos-niri-bar"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

log_info "Fetching latest release binary from GitHub..."

DOWNLOADED=0
# Method A: GitHub CLI (if installed and authenticated)
if command -v gh >/dev/null 2>&1; then
    if gh release download --repo "${REPO}" --pattern "cos-niri-bar" --dir "${TEMP_DIR}" --clobber >/dev/null 2>&1; then
        DOWNLOADED=1
    fi
fi

# Method B: Direct GitHub Releases API
if [ "$DOWNLOADED" -eq 0 ]; then
    RELEASE_JSON=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest")
    DOWNLOAD_URL=$(echo "$RELEASE_JSON" | grep -o 'https://[^"]*releases/download/[^"]*/cos-niri-bar' | head -n 1)

    if [ -n "$DOWNLOAD_URL" ]; then
        curl -sSL -o "${TEMP_DIR}/cos-niri-bar" "$DOWNLOAD_URL"
        DOWNLOADED=1
    fi
fi

if [ "$DOWNLOADED" -eq 1 ] && [ -f "${TEMP_DIR}/cos-niri-bar" ]; then
    cp "${TEMP_DIR}/cos-niri-bar" "${TARGET_BIN}"
    chmod +x "${TARGET_BIN}"
    log_success "Binary installed to: ${TARGET_BIN}"
else
    log_error "Failed to download release binary from GitHub."
    exit 1
fi

# ------------------------------------------------------------------------------
# 3. Install Fonts (Material Symbols Rounded & Roboto)
# ------------------------------------------------------------------------------
log_info "Installing fonts into ${INSTALL_FONT_DIR}..."
mkdir -p "${INSTALL_FONT_DIR}"

FONTS=(
    "MaterialSymbolsRounded.ttf"
    "Roboto-Regular.ttf"
    "Roboto-Medium.ttf"
    "Roboto-Bold.ttf"
)

for font in "${FONTS[@]}"; do
    if [ "$IS_LOCAL_REPO" -eq 1 ] && [ -f "${REPO_DIR}/fonts/${font}" ]; then
        cp "${REPO_DIR}/fonts/${font}" "${INSTALL_FONT_DIR}/${font}"
    else
        log_info "Downloading ${font}..."
        curl -sSL --retry 3 --retry-delay 1 -o "${INSTALL_FONT_DIR}/${font}" "${GITHUB_RAW}/fonts/${font}"
    fi
done

log_info "Updating font cache..."
if command -v fc-cache >/dev/null 2>&1; then
    fc-cache -f "${INSTALL_FONT_DIR}" >/dev/null 2>&1
    log_success "Font cache updated successfully."
fi

# ------------------------------------------------------------------------------
# 4. Initialize Configuration Directory
# ------------------------------------------------------------------------------
mkdir -p "${CONFIG_DIR}"
if [ ! -f "${CONFIG_DIR}/colors.css" ]; then
    log_info "Creating default theme colors at ${CONFIG_DIR}/colors.css..."
    cat > "${CONFIG_DIR}/colors.css" << 'EOF'
@define-color primary #b4c5ff;
@define-color on-primary #1a1b38;
@define-color primary-container rgba(180, 197, 255, 0.14);
@define-color on-primary-container #d0bcff;
@define-color surface rgba(18, 19, 26, 0.72);
@define-color surface-variant rgba(255, 255, 255, 0.07);
@define-color outline rgba(255, 255, 255, 0.10);
@define-color text-primary #ffffff;
@define-color text-secondary #c4c6d0;
@define-color text-muted #938f99;
EOF
fi

if [ ! -f "${CONFIG_DIR}/fuzzel-colors.ini" ]; then
    log_info "Creating default Fuzzel palette at ${CONFIG_DIR}/fuzzel-colors.ini..."
    cat > "${CONFIG_DIR}/fuzzel-colors.ini" << 'EOF'
[colors]
background=12131aff
text=ffffffff
match=b4c5ffff
selection=252836ff
selection-text=b4c5ffff
selection-match=d0bcffff
border=353846ff
EOF
fi

if [ ! -f "${CONFIG_DIR}/settings.json" ]; then
    log_info "Creating default Settings config at ${CONFIG_DIR}/settings.json..."
    cat > "${CONFIG_DIR}/settings.json" << 'EOF'
{
  "theme": {
    "scheme": "scheme-tonal-spot",
    "dark_mode": true
  },
  "performance": {
    "renderer": "vulkan",
    "launcher_backend": "builtin"
  },
  "pinned_apps": [
    { "name": "Google Chrome", "desktop_id": "google-chrome.desktop" },
    { "name": "Firefox", "desktop_id": "firefox.desktop" },
    { "name": "Files", "desktop_id": "org.gnome.Nautilus.desktop" },
    { "name": "VS Code", "desktop_id": "code.desktop" },
    { "name": "Telegram", "desktop_id": "org.telegram.desktop.desktop" }
  ]
}
EOF
fi

# Ensure ~/.config/background exists so swaybg and Settings preview have an initial image
if [ ! -f "$HOME/.config/background" ]; then
    SYSTEM_WALLPAPER=$(find /usr/share/backgrounds /usr/share/wallpapers -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.webp" \) 2>/dev/null | head -n 1)
    if [ -n "$SYSTEM_WALLPAPER" ]; then
        log_info "Setting initial wallpaper from ${SYSTEM_WALLPAPER}..."
        cp "$SYSTEM_WALLPAPER" "$HOME/.config/background"
    fi
fi

# ------------------------------------------------------------------------------
# 5. Backup Old Niri Configuration & Deploy COS Niri Configs
# ------------------------------------------------------------------------------
mkdir -p "${NIRI_DIR}"

# Check if there are existing configuration files to backup
EXISTING_FILES=$(find "${NIRI_DIR}" -maxdepth 1 -name "*.kdl" 2>/dev/null | wc -l)
if [ "$EXISTING_FILES" -gt 0 ]; then
    log_info "Backing up existing Niri configuration to ${NIRI_DIR}/backup.zip..."
    python3 -c '
import os, zipfile
niri_dir = os.path.expanduser("~/.config/niri")
zip_path = os.path.join(niri_dir, "backup.zip")
tmp_zip = os.path.expanduser("~/.config/niri_backup_tmp.zip")
with zipfile.ZipFile(tmp_zip, "w", zipfile.ZIP_DEFLATED) as zipf:
    for root, dirs, files in os.walk(niri_dir):
        for file in files:
            if file in ("backup.zip", "backup.zip.tmp"):
                continue
            full_path = os.path.join(root, file)
            rel_path = os.path.relpath(full_path, niri_dir)
            zipf.write(full_path, rel_path)
os.replace(tmp_zip, zip_path)
' 2>/dev/null || true
    log_success "Backup created at ${NIRI_DIR}/backup.zip"
fi

log_info "Deploying COS Niri configuration files..."
mkdir -p "${NIRI_DIR}/scripts"
mkdir -p "$HOME/.config/fuzzel"
mkdir -p "$HOME/.local/share/nautilus/scripts"

if [ "$IS_LOCAL_REPO" -eq 1 ]; then
    # Local installation from cloned repo
    cp -r "${REPO_DIR}/.config/niri/"* "${NIRI_DIR}/"
    [ -d "${REPO_DIR}/.config/cos-niri" ] && cp -r "${REPO_DIR}/.config/cos-niri/"* "${CONFIG_DIR}/"
    [ -d "${REPO_DIR}/.config/fuzzel" ] && cp -r "${REPO_DIR}/.config/fuzzel/"* "$HOME/.config/fuzzel/"
    if [ -d "${REPO_DIR}/.config/nautilus/scripts" ]; then
        cp -r "${REPO_DIR}/.config/nautilus/scripts/"* "$HOME/.local/share/nautilus/scripts/"
    fi
else
    # Remote standalone installation (curl | bash)
    log_info "Fetching Niri configs from GitHub..."
    NIRI_FILES=("animations.kdl" "binds.kdl" "colors.kdl" "config.kdl" "environment.kdl" "input.kdl" "layout.kdl" "misc.kdl" "outputs.kdl" "rules.kdl" "startup.kdl")
    for f in "${NIRI_FILES[@]}"; do
        curl -sSL --retry 3 --retry-delay 1 -o "${NIRI_DIR}/${f}" "${GITHUB_RAW}/.config/niri/${f}"
    done
    curl -sSL --retry 3 --retry-delay 1 -o "${NIRI_DIR}/scripts/fuzzel-power.sh" "${GITHUB_RAW}/.config/niri/scripts/fuzzel-power.sh"
    curl -sSL --retry 3 --retry-delay 1 -o "$HOME/.config/fuzzel/fuzzel.ini" "${GITHUB_RAW}/.config/fuzzel/fuzzel.ini"
    curl -sSL --retry 3 --retry-delay 1 -o "$HOME/.local/share/nautilus/scripts/Set as Wallpaper" "${GITHUB_RAW}/.config/nautilus/scripts/Set%20as%20Wallpaper"
fi

# Ensure executable permissions on all scripts
chmod +x "${NIRI_DIR}/scripts/"* 2>/dev/null || true
chmod +x "$HOME/.local/share/nautilus/scripts/"* 2>/dev/null || true

log_success "Niri configs deployed to ${NIRI_DIR}"

# ------------------------------------------------------------------------------
# 6. Check PATH & Finish
# ------------------------------------------------------------------------------
echo ""
log_success "Installation and configuration completed successfully!"

if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    log_warn "'$HOME/.local/bin' is not currently in your PATH."
    if [ -n "$FISH_VERSION" ] || [ -f "$HOME/.config/fish/config.fish" ]; then
        echo -e "  ${YELLOW}Fish shell:${NC} run  ${BOLD}fish_add_path ~/.local/bin${NC}\n"
    fi
    echo -e "  ${YELLOW}Bash/Zsh:${NC} add to ~/.bashrc or ~/.zshrc:\n  ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"\n"
fi

echo -e "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BOLD}${GREEN}  Setup Complete!${NC}"
echo -e "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "• Previous Niri config backup: ${BOLD}${NIRI_DIR}/backup.zip${NC}"
echo -e "• New COS Niri configuration:  ${BOLD}${NIRI_DIR}/${NC}"
echo -e "• Fuzzel Launcher config:      ${BOLD}$HOME/.config/fuzzel/fuzzel.ini${NC}"
echo -e "• Dynamic theme colors:        ${BOLD}${CONFIG_DIR}/colors.css${NC}"
echo -e "• Fonts installed:             ${BOLD}${INSTALL_FONT_DIR}/${NC}"
echo -e "• Binary installed:            ${BOLD}${TARGET_BIN}${NC}\n"

echo -e "${BOLD}Next Steps to Apply Changes:${NC}"
echo -e "  1. ${YELLOW}Log out and log back in${NC} (Recommended to initialize all startup services and session rules)"
echo -e "  2. Or apply immediately in your current session:"
echo -e "       ${BLUE}niri msg action load-config-file && cos-niri-bar &${NC}\n"
