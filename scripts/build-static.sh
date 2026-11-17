#!/usr/bin/env bash
# Builds the static musl binary into dist/ and checks that it has no
# dynamic dependencies. Optional: --containers runs it in Alpine and Debian
# (docker or podman, images must be available locally or pullable).
set -euo pipefail
cd "$(dirname "$0")/.."
target=x86_64-unknown-linux-musl
cargo build --release --locked --target "$target"
bin="target/$target/release/ramas"
mkdir -p dist
cp "$bin" dist/ramas
./dist/ramas --man > dist/ramas.1
if ldd dist/ramas 2>&1 | grep -Eq "statically linked|not a dynamic executable"; then
    echo "static: ok"
else
    echo "dist/ramas is dynamically linked:" >&2
    ldd dist/ramas >&2
    exit 1
fi
if [[ "${1:-}" == "--containers" ]]; then
    rt=$(command -v docker || command -v podman)
    for img in alpine:latest debian:stable-slim; do
        echo "== $img"
        "$rt" run --rm -v "$PWD/dist/ramas:/usr/local/bin/ramas:ro" "$img" \
            sh -c 'ramas --version && ramas --export /tmp/s.json /etc'
    done
fi
ls -la dist/
