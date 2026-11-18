#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s):$(uname -m)" != Darwin:arm64 ]]; then
    echo "Bygget kräver en Mac med Apple Silicon." >&2
    exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
source_dir="${FFMPEG_SOURCE_DIR:-$root/target/macos-sources}"
output_dir="${OUTPUT_DIR:-$root/dist}"
work="$(mktemp -d /tmp/motessandare-macos.XXXXXXXX)"
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$source_dir" "$output_dir"
source_dir="$(cd "$source_dir" && pwd)"
output_dir="$(cd "$output_dir" && pwd)"
prefix="$work/deps"
jobs="$(sysctl -n hw.logicalcpu)"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
export CC="$(xcrun --find clang)"
export PKG_CONFIG_PATH=""
export PKG_CONFIG_LIBDIR="$prefix/lib/pkgconfig"

fetch() {
    local name="$1" hash="$2" url="$3"
    if [[ ! -f "$source_dir/$name" ]]; then
        curl --fail --show-error --silent --location --retry 3 "$url" -o "$work/$name"
        (cd "$work" && printf '%s  %s\n' "$hash" "$name" | shasum -a 256 -c -)
        mv "$work/$name" "$source_dir/$name"
    fi
    (cd "$source_dir" && printf '%s  %s\n' "$hash" "$name" | shasum -a 256 -c -)
    tar xf "$source_dir/$name" -C "$work"
}

if ! command -v pkg-config >/dev/null; then
    echo "Installera pkg-config för att bygga FFmpeg." >&2
    exit 1
fi

opus=1.6.1
lame=4.0
openssl=3.5.9
ffmpeg=8.1.3
fetch "opus-$opus.tar.gz" 6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1 \
    "https://downloads.xiph.org/releases/opus/opus-$opus.tar.gz"
fetch "lame-$lame.tar.gz" 3df5124d5ad3a98312ffd7ba6a9b36230e4f8a3e66d3ce0f425e336c32d216eb \
    "https://downloads.sourceforge.net/project/lame/lame/$lame/lame-$lame.tar.gz"
fetch "openssl-$openssl.tar.gz" 603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a \
    "https://github.com/openssl/openssl/releases/download/openssl-$openssl/openssl-$openssl.tar.gz"
fetch "ffmpeg-$ffmpeg.tar.xz" 7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3 \
    "https://ffmpeg.org/releases/ffmpeg-$ffmpeg.tar.xz"

(
    cd "$work/opus-$opus"
    ./configure --prefix="$prefix" --disable-shared --enable-static --disable-doc --disable-extra-programs
    make -j "$jobs"
    make install
)
(
    cd "$work/lame-$lame"
    ./configure --prefix="$prefix" --disable-shared --enable-static --disable-frontend
    make -j "$jobs"
    make install
)
(
    cd "$work/openssl-$openssl"
    perl Configure darwin64-arm64-cc no-shared no-module no-tests \
        --prefix="$prefix" --libdir=lib --openssldir=/private/etc/ssl
    make -j "$jobs"
    make install_sw
)
(
    cd "$work/ffmpeg-$ffmpeg"
    ./configure \
        --disable-autodetect --disable-everything --disable-shared --enable-static \
        --disable-doc --disable-ffplay --disable-ffprobe --disable-avdevice \
        --enable-version3 --enable-small \
        --enable-libopus --enable-libmp3lame --enable-openssl \
        --pkg-config-flags=--static \
        --extra-cflags="-I$prefix/include" --extra-ldflags="-L$prefix/lib" \
        --enable-protocol=file,pipe,http,https,httpproxy,icecast,tcp,tls \
        --enable-demuxer=pcm_f32le \
        --enable-decoder=pcm_f32le \
        --enable-muxer=ogg,mp3,wav,flac \
        --enable-encoder=libopus,libmp3lame,pcm_s16le,pcm_s24le,flac \
        --enable-filter=abuffer,abuffersink,aformat,anull,aresample,atrim
    make -j "$jobs" ffmpeg
)

encoder="$work/ffmpeg-$ffmpeg/ffmpeg"
check_binary() {
    lipo -verify_arch arm64 "$1"
    otool -L "$1" | awk 'NR > 1 && $1 !~ /^\/(System\/Library|usr\/lib)\// {
        print "Oväntat externt bibliotek: " $1 > "/dev/stderr"; bad = 1
    } END { exit bad }'
}

check_binary "$encoder"
"$encoder" -hide_banner -protocols > "$work/protocols.txt"
for protocol in icecast https tls; do
    grep -Eq "^[[:space:]]+$protocol$" "$work/protocols.txt"
done

# Koda korta prov med samma PCM-ingång och samma format som appen använder.
for spec in 'libopus ogg' 'libmp3lame mp3' 'pcm_s16le wav' 'pcm_s24le wav' 'flac flac'; do
    read -r codec format <<< "$spec"
    dd if=/dev/zero bs=4096 count=8 2>/dev/null |
        "$encoder" -hide_banner -loglevel error -nostdin \
        -f f32le -ar 48000 -ac 2 -i pipe:0 -c:a "$codec" -f "$format" "$work/test-$codec"
    test -s "$work/test-$codec"
done

if [[ "${1:-}" == --ffmpeg-only ]]; then
    install -m 755 "$encoder" "$output_dir/ffmpeg"
    exit 0
fi

version="$(sed -nE 's/^version = "([0-9.]+)"/\1/p' "$root/Cargo.toml" | head -n 1)"
test -n "$version"
export CARGO_TARGET_DIR="$root/target"
(cd "$root" && cargo build --locked --release --target aarch64-apple-darwin)

app="$work/Mötessändare.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/Licenser"
install -m 755 "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/motessandare" "$app/Contents/MacOS/motessandare"
install -m 755 "$encoder" "$app/Contents/MacOS/ffmpeg"
licenses="$app/Contents/Resources/Licenser"
cp "$work/ffmpeg-$ffmpeg/COPYING.LGPLv3" "$licenses/FFmpeg-LGPLv3.txt"
cp "$work/ffmpeg-$ffmpeg/COPYING.GPLv3" "$licenses/FFmpeg-GPLv3.txt"
cp "$work/lame-$lame/COPYING" "$licenses/LAME.txt"
cp "$work/opus-$opus/COPYING" "$licenses/Opus.txt"
cp "$work/openssl-$openssl/LICENSE.txt" "$licenses/OpenSSL.txt"
cat > "$licenses/README.txt" <<'NOTICE'
Mötessändare använder FFmpeg som ett separat program.
FFmpeg är byggt med LAME, Opus och OpenSSL.
Källkod och byggskript följer med i ffmpeg-macos-arm64-source.tar.
NOTICE

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>sv</string>
    <key>CFBundleDisplayName</key><string>Mötessändare</string>
    <key>CFBundleExecutable</key><string>motessandare</string>
    <key>CFBundleIdentifier</key><string>se.motessandare.app</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>Mötessändare</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>$MACOSX_DEPLOYMENT_TARGET</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>Mötessändare behöver tillgång till ljudingångarna för att sända och spela in mötets ljud.</string>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist"
check_binary "$app/Contents/MacOS/motessandare"
codesign --force --sign - "$app/Contents/MacOS/ffmpeg"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

# Leverera motsvarande källkod och skript som kan användas för att bygga FFmpeg igen.
mkdir -p "$work/source/scripts"
cp "$root/scripts/package-macos.sh" "$work/source/scripts/"
cp "$source_dir/opus-$opus.tar.gz" "$source_dir/lame-$lame.tar.gz" \
    "$source_dir/openssl-$openssl.tar.gz" "$source_dir/ffmpeg-$ffmpeg.tar.xz" "$work/source/"
cat > "$work/source/README.txt" <<'README'
På en Mac med Apple Silicon, Xcode Command Line Tools och pkg-config:
FFMPEG_SOURCE_DIR="$PWD" OUTPUT_DIR="$PWD" bash scripts/package-macos.sh --ffmpeg-only
README
tar -cf "$work/ffmpeg-macos-arm64-source.tar" -C "$work/source" .
ditto -c -k --keepParent "$app" "$work/motessandare-macos-arm64.zip"
mv -f "$work/ffmpeg-macos-arm64-source.tar" "$work/motessandare-macos-arm64.zip" "$output_dir/"
printf 'Paket och källkod finns i %s\n' "$output_dir"
