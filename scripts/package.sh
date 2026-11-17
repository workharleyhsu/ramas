#!/usr/bin/env bash
# Builds everything a release ships into dist/:
#
#   ramas-<ver>-x86_64-linux.tar.gz (+ .sha256)   static binary, man page, docs
#   ramas-x86_64-linux.tar.gz (+ .sha256)         same, fixed name (install.sh)
#   ramas_<ver>-1_amd64.deb                        Debian, Ubuntu and derivatives
#   ramas-<ver>-1.x86_64.rpm                       Fedora, RHEL, openSUSE
#   SHA256SUMS                                     checksums of all of the above
#
# All three come from the same static (musl) binary, so none depends on the
# distribution's glibc. `--containers` also runs the binary in Alpine and
# Debian (docker or podman; images must be available).
#
# Needs:
#   rustup target add x86_64-unknown-linux-musl
#   cargo install cargo-deb cargo-generate-rpm
set -euo pipefail

cd "$(dirname "$0")/.."
TARGET=x86_64-unknown-linux-musl
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME="ramas-$VERSION-x86_64-linux"
DIST=dist

cargo build --release --locked --target "$TARGET"
BIN="target/$TARGET/release/ramas"

# The published binary must not depend on system libraries.
if ldd "$BIN" 2>&1 | grep -q '=>'; then
    echo "error: $BIN is not static" >&2
    exit 1
fi
# The committed man page must match the CLI.
if ! "$BIN" --man | cmp -s - man/ramas.1; then
    echo "error: man/ramas.1 is stale: cargo run -- --man > man/ramas.1" >&2
    exit 1
fi

rm -rf "${DIST:?}"
mkdir -p "$DIST/$NAME"

# 1. Tarball (used by install.sh).
cp "$BIN" man/ramas.1 README.md README.es.md LICENSE-MIT LICENSE-APACHE "$DIST/$NAME/"
tar -C "$DIST" --owner=0 --group=0 --numeric-owner -czf "$DIST/$NAME.tar.gz" "$NAME"
rm -rf "${DIST:?}/$NAME"
(cd "$DIST" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
# Fixed-name copy: install.sh downloads it from releases/latest/download
# without having to know the version.
cp "$DIST/$NAME.tar.gz" "$DIST/ramas-x86_64-linux.tar.gz"
(cd "$DIST" && sha256sum ramas-x86_64-linux.tar.gz > ramas-x86_64-linux.tar.gz.sha256)

# 2. Packages. With --target, the target/release paths in Cargo.toml point
#    at the musl binary built above.
cargo deb --target "$TARGET" --no-build --no-strip --output "$DIST/"
cargo generate-rpm --target "$TARGET" --output "$DIST/"

# 3. Checksums of everything.
(cd "$DIST" && sha256sum -- *.tar.gz *.deb *.rpm > SHA256SUMS)

if [[ "${1:-}" == "--containers" ]]; then
    rt=$(command -v docker || command -v podman)
    for img in alpine:latest debian:stable-slim; do
        echo "== $img"
        "$rt" run --rm -v "$PWD/$BIN:/usr/local/bin/ramas:ro" "$img" \
            sh -c 'ramas --version && ramas --export /tmp/s.json /etc'
    done
fi

ls -l "$DIST"
