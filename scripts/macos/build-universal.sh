#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repository_root"

toolchain="${SALMON_TOOLCHAIN:-1.88.0}"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/salmon-cli/Cargo.toml | head -n 1)"
package_name="salmon-${version}-macos-universal"
output_dir="${1:-$repository_root/dist}"
targets=(aarch64-apple-darwin x86_64-apple-darwin)

export MACOSX_DEPLOYMENT_TARGET=11.0

for target in "${targets[@]}"; do
    cargo "+$toolchain" build --release --locked -p salmon-cli --target "$target"
done

staging="$output_dir/$package_name"
rm -rf "$staging" "$output_dir/$package_name.tar.gz" "$output_dir/$package_name.tar.gz.sha256"
mkdir -p "$staging/examples"

lipo -create -output "$staging/salmon" \
    "target/aarch64-apple-darwin/release/salmon" \
    "target/x86_64-apple-darwin/release/salmon"
codesign --force --sign - "$staging/salmon"

lipo "$staging/salmon" -verify_arch arm64 x86_64
codesign --verify --strict "$staging/salmon"
for architecture in arm64 x86_64; do
    minimum_os="$(vtool -arch "$architecture" -show-build "$staging/salmon" | awk '/minos/ {print $2}')"
    if [[ "$minimum_os" != "$MACOSX_DEPLOYMENT_TARGET" ]]; then
        echo "unexpected minimum macOS for $architecture: $minimum_os" >&2
        exit 1
    fi
done

cp README.md "$staging/"
cp examples/*.salmon "$staging/examples/"

tar -C "$output_dir" -czf "$output_dir/$package_name.tar.gz" "$package_name"
(cd "$output_dir" && shasum -a 256 "$package_name.tar.gz" > "$package_name.tar.gz.sha256")

echo "$output_dir/$package_name.tar.gz"
