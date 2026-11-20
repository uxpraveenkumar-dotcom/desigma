#!/usr/bin/env bash
# Build a native, locally signed Design App bundle from the Desigma fork.
set -euo pipefail

if [[ $(uname -s) != Darwin ]]; then
    echo "This build script must run on macOS." >&2
    exit 1
fi

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
packaging="$root/packaging/design-app"
build_dir="$root/target/design-app"
mkdir -p "$root/dist"

cargo build --locked --release --target-dir "$build_dir" \
    -p desigma-app --features design-app --bin desigma

output=$(mktemp -d "/private/tmp/design-app.XXXXXX")
trap 'rm -rf "$output"' EXIT
app="$output/Design App.app"
macos="$app/Contents/MacOS"
resources="$app/Contents/Resources"
iconset="$output/DesignApp.iconset"
mkdir -p "$macos" "$resources/licenses" "$iconset"

install -m 755 "$build_dir/release/desigma" "$macos/design-app"
cp "$packaging/Info.plist" "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$packaging/icon.png" \
        --out "$iconset/icon_${size}x${size}.png" >/dev/null
    doubled=$((size * 2))
    sips -z "$doubled" "$doubled" "$packaging/icon.png" \
        --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$resources/DesignApp.icns"
rm -r "$iconset"

cp LICENSE-MIT LICENSE-APACHE "$resources/licenses/"
cp crates/desigma-fonts/assets/inter/OFL.txt "$resources/licenses/Inter-OFL.txt"
cp crates/desigma-app/assets/icons/lucide/LICENSE "$resources/licenses/Lucide-LICENSE.txt"

plutil -lint "$app/Contents/Info.plist"
xattr -cr "$app"
codesign --sign - "$app"
codesign --verify --strict "$app"
trap - EXIT
printf 'Built %s\n' "$app"
