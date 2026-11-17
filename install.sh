#!/usr/bin/env sh
# Installs ramas from this source tree.
#
#   ./install.sh                 install into ~/.local (bin/ and share/man/man1/)
#   ./install.sh --prefix /usr/local
#   ./install.sh --uninstall [--prefix DIR]
#
# Uses dist/ramas when present (scripts/build-static.sh), otherwise builds
# with cargo: the static musl target if installed, else the host target.
set -eu

prefix="${PREFIX:-$HOME/.local}"
action=install
while [ $# -gt 0 ]; do
    case "$1" in
        --prefix) prefix="$2"; shift 2 ;;
        --prefix=*) prefix="${1#--prefix=}"; shift ;;
        --uninstall) action=uninstall; shift ;;
        -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

bin_dir="$prefix/bin"
man_dir="$prefix/share/man/man1"

if [ "$action" = uninstall ]; then
    rm -f "$bin_dir/ramas" "$man_dir/ramas.1"
    echo "removed $bin_dir/ramas and $man_dir/ramas.1"
    exit 0
fi

cd "$(dirname "$0")"
if [ -x dist/ramas ]; then
    bin=dist/ramas
else
    command -v cargo >/dev/null || { echo "cargo not found and dist/ramas missing" >&2; exit 1; }
    musl=x86_64-unknown-linux-musl
    if rustup target list --installed 2>/dev/null | grep -qx "$musl"; then
        cargo build --release --locked --target "$musl"
        bin="target/$musl/release/ramas"
    else
        cargo build --release --locked
        bin=target/release/ramas
    fi
fi

mkdir -p "$bin_dir" "$man_dir"
cp "$bin" "$bin_dir/ramas.tmp" && chmod 755 "$bin_dir/ramas.tmp" && mv "$bin_dir/ramas.tmp" "$bin_dir/ramas"
cp man/ramas.1 "$man_dir/ramas.1" && chmod 644 "$man_dir/ramas.1"
echo "installed $bin_dir/ramas ($("$bin_dir/ramas" --version))"
echo "installed $man_dir/ramas.1"
case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) echo "note: $bin_dir is not in PATH" ;;
esac
