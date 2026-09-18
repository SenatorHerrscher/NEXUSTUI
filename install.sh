#!/usr/bin/env bash
set -e

echo "⚡ Building & Installing NexusTUI Desktop Application..."

# 1. Build release binary
cargo build --release

# 2. Install binary to ~/.local/bin
mkdir -p ~/.local/bin
cp target/release/nexustui ~/.local/bin/nexustui
chmod +x ~/.local/bin/nexustui

# 3. Install launcher wrapper
cat << 'APP_EOF' > ~/.local/bin/nexustui-app
#!/usr/bin/env bash
if ! docker ps --format '{{.Names}}' | grep -q 'ayrik_postgres'; then
    docker start ayrik_postgres >/dev/null 2>&1 || true
fi
if command -v alacritty >/dev/null 2>&1; then
    exec alacritty -T "NexusTUI" --class nexustui,nexustui -e ~/.local/bin/nexustui "$@"
elif command -v konsole >/dev/null 2>&1; then
    exec konsole --qwindowtitle "NexusTUI" -e ~/.local/bin/nexustui "$@"
else
    exec ~/.local/bin/nexustui "$@"
fi
APP_EOF
chmod +x ~/.local/bin/nexustui-app

# 4. Install Icons
mkdir -p ~/.local/share/icons/hicolor/scalable/apps
mkdir -p ~/.local/share/icons/hicolor/256x256/apps
cp assets/nexustui.svg ~/.local/share/icons/hicolor/scalable/apps/nexustui.svg
cp assets/nexustui.png ~/.local/share/icons/hicolor/256x256/apps/nexustui.png

# 5. Install Desktop Entry
mkdir -p ~/.local/share/applications
cp assets/nexustui.desktop ~/.local/share/applications/nexustui.desktop
chmod +x ~/.local/share/applications/nexustui.desktop

# 6. Update Caches
update-desktop-database ~/.local/share/applications >/dev/null 2>&1 || true
kbuildsycoca6 >/dev/null 2>&1 || kbuildsycoca5 >/dev/null 2>&1 || true
gtk-update-icon-cache ~/.local/share/icons/hicolor/ >/dev/null 2>&1 || true

echo "✅ NexusTUI is now installed as a desktop application!"
echo "   You can launch it from your application menu or search with Super."
