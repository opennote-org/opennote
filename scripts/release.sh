#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# Config – change these if your version lives in a different file
# -------------------------------------------------------------------
CARGO_TOML_PATH="crates/opennote-desktop/Cargo.toml"   # <-- adjust if needed
MAIN_BRANCH="main"                                     # or "master"
REMOTE="origin"                                        # remote name

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
# Parse flags
# -------------------------------------------------------------------
DRY_RUN=false
SKIP_CONFIRM=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --yes|-y)
            SKIP_CONFIRM=true
            shift
            ;;
        --*)
            die "Unknown flag: $1"
            ;;
        *)
            break
            ;;
    esac
done

if $DRY_RUN; then
    yellow "=== DRY RUN MODE – no changes will be made ==="
fi

# -------------------------------------------------------------------
# Parse version argument
# -------------------------------------------------------------------
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    die "Usage: $0 [--dry-run] [--yes] <version>   (e.g. $0 1.2.3)"
fi

# Simple semver-ish check
if ! echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    die "Version must be in format X.Y.Z"
fi

TAG="v${VERSION}"

# -------------------------------------------------------------------
# Pre-flight checks
# -------------------------------------------------------------------
echo "$(yellow "→") Running pre-flight checks..."

# 1. Required CLI tools
require_cmd git
require_cmd gh
require_cmd cargo

if ! cargo set-version --version &>/dev/null; then
    die "cargo-edit is required but not installed. Run: cargo install cargo-edit"
fi

# 2. Cargo.toml files exist
WORKSPACE_CARGO="Cargo.toml"
if [ ! -f "$WORKSPACE_CARGO" ]; then
    die "Workspace $WORKSPACE_CARGO not found at project root"
fi
if [ ! -f "$CARGO_TOML_PATH" ]; then
    die "$CARGO_TOML_PATH not found"
fi

# 3. Determine whether we're using workspace inheritance
USE_WORKSPACE=false
if grep -Eq '^[[:space:]]*version[[:space:]]*\.workspace[[:space:]]*=[[:space:]]*true' "$CARGO_TOML_PATH"; then
    USE_WORKSPACE=true
    echo "   Version source: workspace root Cargo.toml (workspace inheritance)"
else
    echo "   Version source: $CARGO_TOML_PATH"
fi

# 4. Parse current version
if $USE_WORKSPACE; then
    CURRENT_VERSION=$(grep -m1 '^[[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]*"' "$WORKSPACE_CARGO" \
        | sed -E 's/.*version[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/')
else
    CURRENT_VERSION=$(grep -m1 '^[[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]*"' "$CARGO_TOML_PATH" \
        | sed -E 's/.*version[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/')
fi

if [ -z "$CURRENT_VERSION" ]; then
    die "Could not parse current version from Cargo.toml"
fi
echo "   Current version: $CURRENT_VERSION"

# 5. Version must be different
if [ "$VERSION" = "$CURRENT_VERSION" ]; then
    die "New version ($VERSION) is the same as current version. No-op."
fi

# 6. Check version is newer (semver comparison – warn only, don't block)
older_version() {
    # Returns 0 if $1 < $2 (semantically)
    local IFS=.
    local i ver1=($1) ver2=($2)
    for ((i=0; i<${#ver1[@]}; i++)); do
        if ((10#${ver1[i]} < 10#${ver2[i]})); then
            return 0
        elif ((10#${ver1[i]} > 10#${ver2[i]})); then
            return 1
        fi
    done
    return 1
}
if ! older_version "$CURRENT_VERSION" "$VERSION"; then
    yellow "   ⚠ Warning: New version ($VERSION) is not greater than current ($CURRENT_VERSION)"
fi

# 7. Must be on the main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "$MAIN_BRANCH" ]; then
    die "You must be on the '$MAIN_BRANCH' branch. Current: $CURRENT_BRANCH"
fi

# 8. Working directory clean
if ! git diff-index --quiet HEAD --; then
    die "Working directory is not clean. Please commit or stash changes first."
fi

# 9. Fetch latest from remote (for accurate up-to-date / tag checks)
echo "   Fetching $REMOTE..."
git fetch "$REMOTE" --tags --quiet

# 10. Local branch is up to date with remote
LOCAL_SHA=$(git rev-parse HEAD)
REMOTE_SHA=$(git rev-parse "$REMOTE/$MAIN_BRANCH" 2>/dev/null || echo "")
if [ -z "$REMOTE_SHA" ]; then
    die "Remote tracking branch '$REMOTE/$MAIN_BRANCH' not found. Does '$REMOTE' exist?"
fi
if [ "$LOCAL_SHA" != "$REMOTE_SHA" ]; then
    LOCAL_AHEAD=$(git rev-list --count "$REMOTE/$MAIN_BRANCH..HEAD")
    REMOTE_AHEAD=$(git rev-list --count "HEAD..$REMOTE/$MAIN_BRANCH")
    if [ "$LOCAL_AHEAD" -gt 0 ] && [ "$REMOTE_AHEAD" -gt 0 ]; then
        die "Branch has diverged from '$REMOTE/$MAIN_BRANCH'. Please sync manually."
    elif [ "$LOCAL_AHEAD" -gt 0 ]; then
        die "Local branch is $LOCAL_AHEAD commit(s) ahead of '$REMOTE/$MAIN_BRANCH'. Push or sync first."
    elif [ "$REMOTE_AHEAD" -gt 0 ]; then
        die "Local branch is $REMOTE_AHEAD commit(s) behind '$REMOTE/$MAIN_BRANCH'. Pull first."
    fi
fi

# 11. Tag must not already exist locally
if git tag -l "$TAG" | grep -qF "$TAG"; then
    die "Tag '$TAG' already exists locally"
fi

# 12. Tag must not already exist on remote
if git ls-remote --tags "$REMOTE" "$TAG" | grep -qF "$TAG"; then
    die "Tag '$TAG' already exists on '$REMOTE'"
fi

# 13. gh CLI authenticated
if ! gh auth status &>/dev/null; then
    die "gh CLI not logged in. Run 'gh auth login' first."
fi

# 14. GitHub release not already present
if gh release view "$TAG" &>/dev/null; then
    die "GitHub release '$TAG' already exists"
fi

echo "$(green "✓") All pre-flight checks passed"

# -------------------------------------------------------------------
# Confirmation prompt
# -------------------------------------------------------------------
if ! $DRY_RUN && ! $SKIP_CONFIRM; then
    echo ""
    echo "   Branch:    $MAIN_BRANCH"
    echo "   Version:   $CURRENT_VERSION → $VERSION"
    echo "   Tag:       $TAG"
    echo ""
    read -r -p "$(yellow "?") Proceed with release? [y/N] " CONFIRM
    case "$CONFIRM" in
        [yY]|[yY][eE][sS]) ;;
        *) die "Aborted by user" ;;
    esac
fi

# -------------------------------------------------------------------
# Dry-run stop
# -------------------------------------------------------------------
if $DRY_RUN; then
    echo ""
    green "✔ Dry-run complete – no changes made. Ready for real release!"
    exit 0
fi

# -------------------------------------------------------------------
# Version bump – uses cargo-edit exclusively
# -------------------------------------------------------------------
echo ""
echo "$(yellow "→") Bumping version to $VERSION"

if $USE_WORKSPACE; then
    cargo set-version "$VERSION" --manifest-path "$WORKSPACE_CARGO"
else
    cargo set-version "$VERSION" --manifest-path "$CARGO_TOML_PATH"
fi

echo "$(green "✓") Version bumped successfully"

# -------------------------------------------------------------------
# Commit, tag, push
# -------------------------------------------------------------------
echo "$(yellow "→") Committing version bump"

# Stage all modified tracked files (catches whichever Cargo.toml changed)
git add -u

# Only commit if there's something staged
if git diff --cached --quiet; then
    die "No changes staged – was the version actually bumped?"
fi

git commit -m "chore: bump version to $VERSION"

echo "$(yellow "→") Creating tag $TAG"
git tag -a "$TAG" -m "Release $TAG"

echo "$(yellow "→") Pushing to $MAIN_BRANCH and tags"
git push "$REMOTE" "$MAIN_BRANCH" --tags

# -------------------------------------------------------------------
# Create GitHub Release
# -------------------------------------------------------------------
echo "$(yellow "→") Creating GitHub Release $TAG (this triggers the build workflow)"
gh release create "$TAG" \
    --title "$TAG" \
    --notes "Release $TAG" \
    --generate-notes

echo ""
echo "$(green "✔") Release $TAG created and published."
echo "   The build workflow will now start and attach the binaries automatically."
