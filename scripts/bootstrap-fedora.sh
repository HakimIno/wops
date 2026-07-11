#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
native_dir="$root_dir/.native"

mkdir -p "$native_dir"
dnf download --arch=x86_64 --destdir="$native_dir" pipewire-devel

rpm_path="$(find "$native_dir" -maxdepth 1 -name 'pipewire-devel-*.x86_64.rpm' -print -quit)"
if [[ -z "$rpm_path" ]]; then
    echo "pipewire-devel RPM was not downloaded" >&2
    exit 1
fi

(
    cd "$native_dir"
    rpm2cpio "$rpm_path" | cpio -idm
)

cp /usr/lib64/libpipewire-0.3.so.0 "$native_dir/usr/lib64/"
for pc_file in \
    "$native_dir/usr/lib64/pkgconfig/libpipewire-0.3.pc" \
    "$native_dir/usr/lib64/pkgconfig/libspa-0.2.pc"
do
    sed -i "s|^prefix=/usr$|prefix=$native_dir/usr|" "$pc_file"
done

echo "Local PipeWire SDK is ready at $native_dir"
