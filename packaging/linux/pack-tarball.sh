#!/bin/sh
# Build the release binary and pack dist/gantry-<version>-x86_64-linux.tar.gz (run by build-tarball.sh inside its container).
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
cd "$root"
cargo build --release -p gantry
# gantryd is built static with musl, so it runs on hosts with an older glibc than this build's.
cargo build --release -p gantry --bin gantryd --no-default-features --target x86_64-unknown-linux-musl
stage=$root/dist/gantry-$version
rm -rf "$stage"
mkdir -p "$stage"
# gantryd: Gantry without its window, which the SSH wizard uploads to remote hosts.
install -m0755 "${CARGO_TARGET_DIR:-target}/release/gantry" \
  "${CARGO_TARGET_DIR:-target}/x86_64-unknown-linux-musl/release/gantryd" packaging/linux/install.sh "$stage/"
install -m0644 packaging/linux/io.github.idogrf.Gantry.desktop packaging/linux/io.github.idogrf.Gantry.svg \
  LICENSE-MIT LICENSE-APACHE NOTICE THIRD_PARTY_LICENSES.html "$stage/"
# The embedded interface font's licence.
install -m0644 crates/ui-gpui/assets/fonts/OFL.txt "$stage/LICENSE-FONT-FIGTREE"
tar -C "$root/dist" -czf "$root/dist/gantry-$version-x86_64-linux.tar.gz" "gantry-$version"
echo "$root/dist/gantry-$version-x86_64-linux.tar.gz"
