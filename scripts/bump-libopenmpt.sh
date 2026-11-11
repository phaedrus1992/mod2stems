#!/usr/bin/env bash
# Move the vendored libopenmpt pin to a release tag and regenerate bindings.
#
# Usage:
#   scripts/bump-libopenmpt.sh <tag>    for example: libopenmpt-0.8.11
#   scripts/bump-libopenmpt.sh --check-tag <tag>
#   scripts/bump-libopenmpt.sh --help
#
# --check-tag validates a tag only. It prints the three version fields, one per
# line, and exits 1 for a tag that is not a release. It reads no file and
# touches no git state, so it can run in a test.
#
# The tag must name a release. The script checks out the tag in the submodule,
# reads its commit, SVN revision, URL, and date, and writes them to the five
# pin constants in crates/openmpt-ext-sys/build.rs. It then runs
# scripts/regen-bindings.sh.
#
# stdout holds one block of "old" and "new" values. Diagnostics go to stderr.
# A failure leaves the tree as it was, where a failure is possible before the
# first write.
set -euo pipefail

# Prints the comment header. It stops at the first line that is not a comment,
# so a header edit cannot leak a shell statement into the help text.
usage() {
	awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"
}

die() {
	echo "bump-libopenmpt.sh: $1" >&2
	exit 1
}

note() {
	echo "bump-libopenmpt.sh: $1" >&2
}

# The release-tag pattern. Each field is one to three digits, so a suffix such
# as "rc1" and a missing field are both rejected.
TAG_PATTERN='^libopenmpt-([0-9]{1,3})\.([0-9]{1,3})\.([0-9]{1,3})$'

# Rejects a tag that is not a release tag.
check_tag() {
	[[ "$1" =~ $TAG_PATTERN ]] ||
		die "the tag '$1' is not a release tag. Use libopenmpt-<major>.<minor>.<patch>, for example libopenmpt-0.8.11"
}

case "${1:-}" in
--help | -h)
	usage
	exit 0
	;;
--check-tag)
	tag="${2:-}"
	[[ -n "$tag" ]] || die "--check-tag needs a tag. Run: scripts/bump-libopenmpt.sh --check-tag libopenmpt-0.8.10"
	check_tag "$tag"
	# The match above sets BASH_REMATCH, which holds the three fields.
	printf '%s\n%s\n%s\n' "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}"
	exit 0
	;;
"")
	usage >&2
	die "a tag is required. Run: scripts/bump-libopenmpt.sh libopenmpt-0.8.11"
	;;
esac

tag="$1"
check_tag "$tag"

ROOT="$(git rev-parse --show-toplevel)"
VENDOR="$ROOT/crates/openmpt-ext-sys/vendor/openmpt"
BUILD_RS="$ROOT/crates/openmpt-ext-sys/build.rs"

[[ -f "$VENDOR/libopenmpt/libopenmpt.h" ]] ||
	die "$VENDOR/libopenmpt/libopenmpt.h is missing. Run: git submodule update --init --depth 1"
[[ -f "$BUILD_RS" ]] || die "$BUILD_RS is missing. Run the script from the repository."

command -v bindgen >/dev/null 2>&1 ||
	die "bindgen is not on PATH. Run: cargo install bindgen-cli --version 0.73.2 --locked"

old_tag="$(sed -nE 's/^const PIN_TAG: &str = "(.*)";$/\1/p' "$BUILD_RS")"
old_commit="$(sed -nE 's/^const PIN_COMMIT: &str = "(.*)";$/\1/p' "$BUILD_RS")"
old_revision="$(sed -nE 's/^const PIN_SVN_REVISION: &str = "(.*)";$/\1/p' "$BUILD_RS")"

note "fetching $tag"
# A release tag can be fetched on its own, because the submodule is shallow.
git -C "$VENDOR" fetch --quiet --depth 1 --force origin "tag" "$tag" ||
	die "cannot fetch the tag $tag from origin. Check the tag name and the network."
git -C "$VENDOR" checkout --quiet --force --detach "refs/tags/$tag" ||
	die "cannot check out the tag $tag"

new_commit="$(git -C "$VENDOR" rev-parse HEAD)"
message="$(git -C "$VENDOR" log -1 --format=%B)"
# A commit with no git-svn-id line makes grep exit 1. Under `pipefail` that
# aborts the script before the guard below can name the cause.
svn_line="$(grep -F 'git-svn-id:' <<<"$message" | head -n 1)" || svn_line=""
[[ -n "$svn_line" ]] || die "the commit $new_commit holds no git-svn-id line. The revision is not known."
svn_id="${svn_line##*git-svn-id: }"
svn_id="${svn_id%% *}"
new_revision="${svn_id##*@}"
new_url="${svn_id%@*}"
[[ "$new_revision" =~ ^[0-9]{1,7}$ ]] || die "the git-svn-id line '$svn_line' names no revision."
new_date="$(git -C "$VENDOR" log -1 --format=%cI)"

# Writes one string constant. The replacement uses environment variables, so a
# value holds no regular expression syntax of its own.
set_const() {
	local name="$1" value="$2"
	NEW_NAME="$name" NEW_VALUE="$value" perl -pi -e \
		's{^(const \Q$ENV{NEW_NAME}\E: &str = ")[^"]*(";)$}{$1$ENV{NEW_VALUE}$2}' "$BUILD_RS"
	grep -qF "const $name: &str = \"$value\";" "$BUILD_RS" ||
		die "the rewrite of $name did not take effect in $BUILD_RS"
}

set_const PIN_TAG "$tag"
set_const PIN_COMMIT "$new_commit"
set_const PIN_SVN_REVISION "$new_revision"
set_const PIN_SVN_URL "$new_url"
set_const PIN_SVN_DATE "$new_date"

note "regenerating bindings"
"$ROOT/scripts/regen-bindings.sh" >&2 || die "the bindings did not regenerate. Fix the cause, then run: scripts/regen-bindings.sh"

printf 'old tag:      %s\n' "$old_tag"
printf 'old commit:   %s\n' "$old_commit"
printf 'old revision: %s\n' "$old_revision"
printf 'new tag:      %s\n' "$tag"
printf 'new commit:   %s\n' "$new_commit"
printf 'new revision: %s\n' "$new_revision"
printf 'new url:      %s\n' "$new_url"
printf 'new date:     %s\n' "$new_date"
