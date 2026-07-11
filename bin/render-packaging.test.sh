#!/usr/bin/env bash
# Unit test for bin/render-packaging. Renders both manifests from the real
# templates with fixture inputs and asserts the version, checksums, and every
# templated field land correctly — and that bad inputs are rejected rather than
# producing a broken manifest. Fast and hermetic (no network, no makepkg/brew).
#
# Run via `just test`, or directly: bash bin/render-packaging.test.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
render="$here/render-packaging"

fail() { echo "FAIL: $1" >&2; exit 1; }

# 64-char fixture digests, distinct so a swapped field is caught.
deb_sha='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
license_sha='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
dmg_sha='cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc'
ver='1.2.3'

# --- PKGBUILD rendering ---
pkgbuild="$("$render" pkgbuild "$ver" "$deb_sha" "$license_sha")"
grep -qx "pkgver=$ver" <<<"$pkgbuild" || fail "PKGBUILD pkgver not substituted"
grep -qF "'$deb_sha'" <<<"$pkgbuild" || fail "PKGBUILD missing deb sha256"
grep -qF "'$license_sha'" <<<"$pkgbuild" || fail "PKGBUILD missing license sha256"
grep -qx "pkgname=unplot-bin" <<<"$pkgbuild" || fail "PKGBUILD pkgname changed"
grep -qF "provides=(\"unplot=\$pkgver\")" <<<"$pkgbuild" || fail "PKGBUILD provides changed"
# [A-Z0-9_] (not [A-Z_]) so the digit-bearing checksum placeholders are covered.
grep -qE '__[A-Z0-9_]+__' <<<"$pkgbuild" && fail "PKGBUILD has leftover placeholders"

# --- Cask rendering ---
cask="$("$render" cask "$ver" "$dmg_sha")"
grep -qF "version \"$ver\"" <<<"$cask" || fail "cask version not substituted"
grep -qF "sha256 \"$dmg_sha\"" <<<"$cask" || fail "cask missing dmg sha256"
grep -qF 'app "unplot.app"' <<<"$cask" || fail "cask app stanza changed"
grep -qF 'com.apple.quarantine' <<<"$cask" || fail "cask lost the unsigned-app caveat"
grep -qE '__[A-Z0-9_]+__' <<<"$cask" && fail "cask has leftover placeholders"

# Regression guard on the guard: emit()'s placeholder regex must keep digits in
# its class, or the checksum placeholders (…SHA256…) slip through unnoticed.
grep -qF '__[A-Z0-9_]+__' "$render" || fail "emit() placeholder regex must include digits ([A-Z0-9_])"

# --- Input validation: each bad case must exit non-zero ---
"$render" pkgbuild "$ver" "not-a-sha" "$license_sha" 2>/dev/null && fail "accepted a non-sha256 deb digest"
"$render" pkgbuild "vX.Y.Z" "$deb_sha" "$license_sha" 2>/dev/null && fail "accepted a non-version pkgver"
"$render" cask "$ver" "$deb_sha" "extra-arg" 2>/dev/null && fail "accepted wrong cask arg count"
"$render" bogus "$ver" 2>/dev/null && fail "accepted an unknown target"

echo "ok: render-packaging fills every placeholder for pkgbuild + cask and rejects bad inputs"
