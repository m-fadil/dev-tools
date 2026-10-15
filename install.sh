#!/bin/sh
# Pemasang dt: mengunduh binary rilis sesuai OS/arsitektur, memverifikasi checksum,
# lalu memasangnya ke ~/.local/bin.
#
#   curl -fsSL https://github.com/m-fadil/dev-tools/releases/latest/download/install.sh | sh
#   curl -fsSL https://github.com/m-fadil/dev-tools/releases/download/v1.1.0/install.sh | sh
#
# Pilihan versi (yang pertama berlaku): DT_VERSION, argumen (`sh -s -- v1.1.0`),
# versi yang ditanam saat rilis, lalu rilis stabil terbaru.
#
#   DT_VERSION      versi yang dipasang, mis. v1.1.0 atau 1.1.0-rc.1
#   DT_INSTALL_DIR  folder tujuan (default: $HOME/.local/bin)
#   DT_RELEASES_URL dasar URL rilis (default: GitHub Releases; untuk pengujian)

set -eu

REPO="m-fadil/dev-tools"
# Diisi otomatis oleh workflow rilis; kosong berarti rilis stabil terbaru.
DEFAULT_VERSION=""

die() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

have() {
    command -v "$1" >/dev/null 2>&1
}

fetch() {
    if have curl; then
        curl -fsSL --retry 3 -o "$2" "$1"
    elif have wget; then
        wget -q -O "$2" "$1"
    else
        die "butuh curl atau wget."
    fi
}

# Mencetak URL tujuan akhir dari sebuah redirect.
redirect_target() {
    if have curl; then
        curl -fsSLI -o /dev/null -w '%{url_effective}' "$1"
    elif have wget; then
        # wget 1.x prints a Location header; wget2 prints "URL '...' not followed".
        wget -S --spider --max-redirect=0 "$1" 2>&1 |
            sed -n -e 's/^ *[Ll]ocation: *//p' -e "s/^URL '\(.*\)' not followed.*/\1/p" | tr -d '\r' | sed -n '1p'
    else
        die "butuh curl atau wget."
    fi
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$arch" in
        x86_64 | amd64) arch="x86_64" ;;
        aarch64 | arm64) arch="aarch64" ;;
        *) die "arsitektur tidak didukung: $arch" ;;
    esac
    case "$os" in
        Linux) printf '%s-unknown-linux-musl' "$arch" ;;
        Darwin) printf '%s-apple-darwin' "$arch" ;;
        MINGW* | MSYS* | CYGWIN*) die "di Windows, unduh arsip .zip dari halaman rilis: https://github.com/$REPO/releases" ;;
        *) die "sistem operasi tidak didukung: $os" ;;
    esac
}

resolve_version() {
    version="${DT_VERSION:-${1:-$DEFAULT_VERSION}}"
    if [ -z "$version" ]; then
        location=$(redirect_target "$releases/latest") || location=""
        version="${location##*/tag/}"
        case "$location" in
            */tag/v*) ;;
            *) die "tidak ada rilis stabil. Tentukan versi: DT_VERSION=v1.1.0" ;;
        esac
    fi
    case "$version" in
        v*) ;;
        *) version="v$version" ;;
    esac
    printf '%s' "$version"
}

verify() {
    if have sha256sum; then
        sha256sum -c "$1"
    elif have shasum; then
        shasum -a 256 -c "$1"
    else
        die "butuh sha256sum atau shasum untuk memverifikasi unduhan."
    fi
}

main() {
    releases="${DT_RELEASES_URL:-https://github.com/$REPO/releases}"
    install_dir="${DT_INSTALL_DIR:-${HOME:?HOME belum diatur}/.local/bin}"

    target=$(detect_target)
    version=$(resolve_version "${1:-}")
    name="dt-$version-$target"
    archive="$name.tar.gz"

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT INT TERM

    printf 'Memasang dt %s (%s)\n' "$version" "$target"
    fetch "$releases/download/$version/$archive" "$tmp/$archive" ||
        die "gagal mengunduh $releases/download/$version/$archive (versi atau platform tidak ada?)"
    fetch "$releases/download/$version/$archive.sha256" "$tmp/$archive.sha256" ||
        die "gagal mengunduh checksum $archive.sha256"
    (cd "$tmp" && verify "$archive.sha256" >/dev/null) || die "checksum tidak cocok untuk $archive."

    tar -xzf "$tmp/$archive" -C "$tmp" || die "gagal mengekstrak $archive."
    [ -f "$tmp/$name/dt" ] || die "arsip tidak berisi dt."

    mkdir -p "$install_dir"
    cp "$tmp/$name/dt" "$install_dir/.dt.new"
    chmod 755 "$install_dir/.dt.new"
    mv -f "$install_dir/.dt.new" "$install_dir/dt"

    printf 'Terpasang: %s/dt\n' "$install_dir"
    # shellcheck disable=SC2016 # $PATH harus tampil apa adanya
    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *) printf 'Tambahkan ke PATH: export PATH="%s:$PATH"\n' "$install_dir" ;;
    esac
}

main "$@"
