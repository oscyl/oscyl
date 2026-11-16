#!/usr/bin/env bash
# A folder that runs the GTK app on a Windows machine with nothing else
# installed: bin/Oscyl.exe with every DLL it loads from MSYS2, share/ (see
# bundle-share.sh) and licences/ with the texts of the libraries it ships.
# GLib finds share/ from the DLLs' location, so no launcher is needed.
# Usage: bundle-windows.sh EXE PREFIX OUT
#   EXE     the built oscyl-gtk.exe
#   PREFIX  MSYS2's UCRT64 prefix (/ucrt64)
#   OUT     the folder to create
# Needs objdump (or $OBJDUMP), pacman (MSYS2's, for the licences),
# glib-compile-schemas, gtk-update-icon-cache and msgfmt.
set -euo pipefail

exe=$1 prefix=$2 out=$3
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
objdump=${OBJDUMP:-objdump}

mkdir -p "$out/bin" "$out/licences"
cp "$exe" "$out/bin/Oscyl.exe"

# Every DLL the exe imports, and theirs, that MSYS2 provides; the rest are
# Windows' own.
todo=("$out/bin/Oscyl.exe")
while ((${#todo[@]})); do
    file=${todo[-1]}
    unset 'todo[-1]'
    while read -r dll; do
        src=$(find "$prefix/bin" -maxdepth 1 -iname "$dll" -print -quit)
        if [[ -n $src && ! -e $out/bin/${src##*/} ]]; then
            cp "$src" "$out/bin/"
            todo+=("$out/bin/${src##*/}")
        fi
    done < <("$objdump" -p "$file" | sed -n 's/^[[:space:]]*DLL Name: //p')
done

"$here/bundle-share.sh" "$prefix" "$out/share"

# The licence texts of the packages the DLLs came from, and their exact
# versions (the sources are on packages.msys2.org).
cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$out/"
cp "$here/THIRD-PARTY.txt" "$out/licences/"
for dll in "$out"/bin/*.dll; do
    pacman -Qqo "$prefix/bin/${dll##*/}"
done | sort -u | while read -r pkg; do
    pacman -Q "$pkg" >> "$out/licences/PACKAGES.txt"
    pacman -Qlq "$pkg" | grep '/share/licenses/.*[^/]$' | while read -r f; do
        name=${f#*/share/licenses/}
        mkdir -p "$out/licences/${name%/*}"
        cp "$f" "$out/licences/$name"
    done || true
done
