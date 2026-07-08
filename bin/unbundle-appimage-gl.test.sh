#!/usr/bin/env bash
# Unit test for bin/unbundle-appimage-gl's library-selection logic. It exercises
# prune_host_gl_libs against a fixture lib directory — no real AppImage needed, so
# it is fast and hermetic. The squashfs extract/repack plumbing is covered by the
# release pipeline and by manual runs against a built AppImage, not here.
#
# Run via `just test`, or directly: bash bin/unbundle-appimage-gl.test.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=/dev/null
source "$here/unbundle-appimage-gl" # the source guard keeps main() from running

fail() { echo "FAIL: $1" >&2; exit 1; }

tmp="$(mktemp -d)"
# shellcheck disable=SC2064
trap "rm -rf '$tmp'" EXIT
libdir="$tmp/usr/lib"
mkdir -p "$libdir"

# Libraries that MUST be removed — the host-coupled GL/display stack, with the
# realistic soname suffixes a bundle actually carries.
coupled=(
  libwayland-client.so.0 libwayland-egl.so.1 libwayland-cursor.so.0
  libwayland-server.so.0 libepoxy.so.0 libEGL.so.1 libGL.so.1
  libgbm.so.1 libdrm.so.2 libGLdispatch.so.0
)
# Libraries that MUST be kept — bundling these is correct. libwayland-notreal.txt
# guards against an over-broad glob that would nuke anything starting "libwayland".
kept=(
  libwebkit2gtk-4.1.so.0 libgtk-3.so.0 libgstgl-1.0.so.0
  libjavascriptcoregtk-4.1.so.0 libsoup-3.0.so.0 libwayland-notreal.txt
)

for f in "${coupled[@]}" "${kept[@]}"; do : > "$libdir/$f"; done

prune_host_gl_libs "$libdir" >/dev/null

for f in "${coupled[@]}"; do
  [[ -e "$libdir/$f" ]] && fail "coupled lib not removed: $f"
done
for f in "${kept[@]}"; do
  [[ -e "$libdir/$f" ]] || fail "kept lib was wrongly removed: $f"
done

# Regression guard: the confirmed EGL_BAD_PARAMETER trigger on this project's
# AppImage is libwayland-* + libepoxy, so those globs must stay in the set.
for must in libwayland-client.so libwayland-egl.so libwayland-cursor.so \
  libwayland-server.so libepoxy.so; do
  host_coupled_lib_globs | grep -qx "$must" || fail "missing required removal glob: $must"
done

# An empty directory must be handled cleanly (count 0, no error).
empty="$tmp/empty"; mkdir -p "$empty"
[[ "$(prune_host_gl_libs "$empty" | tail -1)" == "0" ]] || fail "empty dir should prune 0 libs"

echo "ok: unbundle-appimage-gl removes the host-coupled GL/display libs and keeps the rest"
