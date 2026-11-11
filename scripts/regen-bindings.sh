#!/usr/bin/env bash
# Regenerate crates/openmpt-ext-sys/src/bindings.rs from the pinned headers.
#
# Usage:
#   scripts/regen-bindings.sh           write src/bindings.rs
#   scripts/regen-bindings.sh --check   fail when the committed file differs
#
# Requires the bindgen CLI at the pinned version (see BINDGEN_VERSION).
set -euo pipefail

BINDGEN_VERSION="0.73.2"

# The vendored tree must be present. A missing header means an unfilled submodule.
ROOT="$(git rev-parse --show-toplevel)"
SYS_CRATE="$ROOT/crates/openmpt-ext-sys"
VENDOR="$SYS_CRATE/vendor/openmpt"
WRAPPER="$SYS_CRATE/wrapper.h"
OUTPUT="$SYS_CRATE/src/bindings.rs"

# Prints the comment header. It stops at the first line that is not a comment,
# so a header edit cannot leak a shell statement into the help text.
usage() {
	awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"
}

die() {
	echo "regen-bindings.sh: $1" >&2
	exit 1
}

mode="write"
case "${1:-}" in
--check) mode="check" ;;
--help | -h)
	usage
	exit 0
	;;
"") ;;
*)
	usage >&2
	die "unknown argument '$1'. Use --check, or no argument."
	;;
esac

command -v bindgen >/dev/null 2>&1 || die "bindgen is not on PATH. Run: cargo install bindgen-cli --version $BINDGEN_VERSION --locked"

# The bindings must come from the pinned generator, or --check flaps between machines.
found="$(bindgen --version | sed -E 's/^bindgen ([0-9.]+).*/\1/')"
[[ "$found" == "$BINDGEN_VERSION" ]] ||
	die "bindgen is $found, but the pin is $BINDGEN_VERSION. Run: cargo install bindgen-cli --version $BINDGEN_VERSION --locked"

[[ -f "$VENDOR/libopenmpt/libopenmpt.h" ]] ||
	die "$VENDOR/libopenmpt/libopenmpt.h is missing. Run: git submodule update --init --depth 1"

# The allowlists keep the output to the public C API and its constants. Layout
# tests stay on, so a header change that moves a struct offset fails a build.
run_bindgen() {
	bindgen "$WRAPPER" \
		--allowlist-function 'openmpt_.*' \
		--allowlist-type 'openmpt_.*' \
		--allowlist-var '(OPENMPT|LIBOPENMPT)_.*' \
		--no-doc-comments \
		--output "$1" \
		-- -I "$VENDOR"
}

if [[ "$mode" == "write" ]]; then
	run_bindgen "$OUTPUT"
	rustfmt --edition 2024 "$OUTPUT"
	echo "wrote $OUTPUT"
	exit 0
fi

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
run_bindgen "$tmp"
rustfmt --edition 2024 "$tmp"

if ! diff -u "$OUTPUT" "$tmp"; then
	die "src/bindings.rs does not match the pinned headers. Run: scripts/regen-bindings.sh"
fi
echo "bindings match the pinned headers"
