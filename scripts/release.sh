#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# Config – change these if your version lives in a different file
# -------------------------------------------------------------------
CARGO_TOML_PATH="crates/opennote-desktop/Cargo.toml"   # <-- adjust if needed
MAIN_BRANCH="main"                                     # or "master"

# -------------------------------------------------------------------
# Helpers
# -------------------------------------------------------------------
red()    { echo -e "\033[31m$*\033[0m"; }
green()  { echo -e "\033[32m$*\033[0m"; }
yellow() { echo -e "\033[33m$*\033[0m"; }

die() { red "ERROR: $*" >&2; exit 1; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || die "Required command '$1' not found. Please install it."
}

# -------------------------------------------------------------------
# Validation
# -------------------------------------------------------------------
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    die "Usage: $0 <version>   (e.g. $0 1.2.3)"
fi

# Simple semver-ish check
if ! echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    die "Version must be in format X.Y.Z"
fi

# Check required tools
require_cmd git
require_cmd gh

# Must be on the main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "$MAIN_BRANCH" ]; then
    die "You must be on the '$MAIN_BRANCH' branch. Current: $CURRENT_BRANCH"
fi

# Working directory clean
if ! git diff-index --quiet HEAD --; then
    die "Working directory is not clean. Please commit or stash changes first."
fi

# gh CLI authenticated
if ! gh auth status &>/dev/null; then
    die "gh CLI not logged in. Run 'gh auth login' first."
fi

# -------------------------------------------------------------------
# Version bump
# -------------------------------------------------------------------
echo "$(yellow "→") Bumping version to $VERSION in $CARGO_TOML_PATH"

if [ ! -f "$CARGO_TOML_PATH" ]; then
    die "File not found: $CARGO_TOML_PATH"
fi

if command -v cargo-set-version &>/dev/null; then
    # Prefer cargo-edit if installed
    cargo set-version "$VERSION" --manifest-path "$CARGO_TOML_PATH"
elif command -v sed &>/dev/null; then
    # Fallback: update first `version = "x.y.z"` line
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML_PATH"
    else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$CARGO_TOML_PATH"
    fi
else
    die "Neither 'cargo set-version' (cargo-edit) nor 'sed' available. Install one of them."
fi

# Verify the version now matches
grep -q "version = \"$VERSION\"" "$CARGO_TOML_PATH" || die "Version string not found after bump. Check $CARGO_TOML_PATH manually."

# -------------------------------------------------------------------
# Commit, tag, push
# -------------------------------------------------------------------
echo "$(yellow "→") Committing version bump"
git add "$CARGO_TOML_PATH"
git commit -m "chore: bump version to $VERSION"

echo "$(yellow "→") Creating tag v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"

echo "$(yellow "→") Pushing to $MAIN_BRANCH and tags"
git push origin "$MAIN_BRANCH" --tags

# -------------------------------------------------------------------
# Create GitHub Release
# -------------------------------------------------------------------
echo "$(yellow "→") Creating GitHub Release v$VERSION (this triggers the build workflow)"
gh release create "v$VERSION" \
    --title "v$VERSION" \
    --notes "Release v$VERSION" \
    --generate-notes

echo ""
echo "$(green "✔") Release v$VERSION created and published."
echo "   The build workflow will now start and attach the binaries automatically."