#!/bin/sh
# No Make, Node, Python, or Git required. Run from any working directory.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

admin() {
    if [ "$(id -u)" = 0 ]; then "$@"; else sudo "$@"; fi
}

case "$(uname -s)" in
    Darwin)
        if ! xcode-select -p >/dev/null 2>&1; then
            echo 'Installing Apple Command Line Tools. Complete the system dialog, then run this launcher again.'
            xcode-select --install
            exit 1
        fi
        ;;
    Linux)
        # Install only missing system prerequisites using the distribution's manager.
        if ! command -v cc >/dev/null 2>&1 || ! command -v curl >/dev/null 2>&1 || ! command -v xdg-open >/dev/null 2>&1; then
            echo 'Installing compiler, HTTPS certificates, curl, and browser opener (may request your password).'
            if command -v apt-get >/dev/null 2>&1; then
                admin apt-get update
                admin apt-get install -y build-essential curl ca-certificates xdg-utils
            elif command -v dnf >/dev/null 2>&1; then
                admin dnf install -y gcc curl ca-certificates xdg-utils
            elif command -v pacman >/dev/null 2>&1; then
                admin pacman -S --needed --noconfirm base-devel curl ca-certificates xdg-utils
            elif command -v zypper >/dev/null 2>&1; then
                admin zypper --non-interactive install gcc curl ca-certificates xdg-utils
            elif command -v apk >/dev/null 2>&1; then
                admin apk add build-base curl ca-certificates xdg-utils
            else
                echo 'Install a C compiler/linker, curl, CA certificates, and xdg-utils using your system package manager, then rerun.' >&2
                exit 1
            fi
        fi
        ;;
    *) echo 'This launcher supports macOS and Linux. On Windows, use windows_launcher.cmd.' >&2; exit 1 ;;
esac

if ! command -v cargo >/dev/null 2>&1 || ! rustc --version >/dev/null 2>&1; then
    echo 'Installing Rust with the official rustup installer.'
    installer=$(mktemp)
    trap 'rm -f "$installer"' EXIT HUP INT TERM
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$installer"
    sh "$installer" -y --profile minimal
    rm -f "$installer"
    trap - EXIT HUP INT TERM
fi
# Leave compatible existing toolchains alone; upgrade old Rust only through rustup.
version=$(rustc --version | cut -d ' ' -f 2)
major=${version%%.*}
minor=${version#*.}; minor=${minor%%.*}
if [ "$major" -lt 1 ] || { [ "$major" -eq 1 ] && [ "$minor" -lt 70 ]; }; then
    if ! command -v rustup >/dev/null 2>&1; then
        echo 'Rust 1.70 or newer is required. Update your system Rust installation and rerun.' >&2
        exit 1
    fi
    rustup toolchain install stable --profile minimal
    export RUSTUP_TOOLCHAIN=stable
fi
cargo build --release --offline
exec target/release/chess-ai --open "$@"
