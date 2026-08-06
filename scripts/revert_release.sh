#!/usr/bin/env bash
set -euo pipefail

# -------------------------------------------------------------------
# Config – keep these aligned with release.sh
# -------------------------------------------------------------------
CARGO_TOML_PATH="crates/opennote-desktop/Cargo.toml"
WORKSPACE_CARGO="Cargo.toml"
MAIN_BRANCH="main"
REMOTE="origin"
RELEASE_WORKFLOW="release.yml"
CANCEL_WAIT_SECONDS=60

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

STAGE="pre-flight checks"
MUTATION_STARTED=false
on_error() {
    local exit_code=$?
    local line=$1
    trap - ERR

    red "ERROR: Rollback failed during: $STAGE (line $line)" >&2
    if $MUTATION_STARTED; then
        yellow "Some remote or local changes may already have completed."
        echo "Inspect the current state before retrying:"
        echo "  gh release view '$TAG'"
        echo "  git ls-remote --tags '$REMOTE' 'refs/tags/$TAG'"
        echo "  git tag -l '$TAG'"
        echo "  git status --short --branch"
        echo "  git log -3 --oneline --decorate"
    fi
    exit "$exit_code"
}
trap 'on_error "$LINENO"' ERR

read_version_from_commit() {
    local commit=$1
    local manifest=$2
    local manifest_contents
    local version

    manifest_contents=$(git show "${commit}:${manifest}") || return 1
    version=$(printf '%s\n' "$manifest_contents" \
        | grep -m1 '^[[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]*"' \
        | sed -E 's/.*version[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/') || return 1

    [ -n "$version" ] || return 1
    printf '%s\n' "$version"
}

is_active_run_status() {
    case "$1" in
        queued|in_progress|waiting|requested|pending) return 0 ;;
        *) return 1 ;;
    esac
}

# -------------------------------------------------------------------
# Parse flags and version
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

VERSION="${1:-}"
if [ -z "$VERSION" ] || [ $# -ne 1 ]; then
    die "Usage: $0 [--dry-run] [--yes] <version>   (e.g. $0 1.2.3)"
fi

if ! echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    die "Version must be in format X.Y.Z"
fi

TAG="v${VERSION}"
EXPECTED_SUBJECT="chore: bump version to $VERSION"

if $DRY_RUN; then
    yellow "=== DRY RUN MODE – no changes will be made ==="
fi

# -------------------------------------------------------------------
# Pre-flight checks
# -------------------------------------------------------------------
echo "$(yellow "→") Running pre-flight checks for $TAG..."

require_cmd git
require_cmd gh

[ -f "$WORKSPACE_CARGO" ] || die "Workspace $WORKSPACE_CARGO not found. Run this script from the project root."
[ -f "$CARGO_TOML_PATH" ] || die "$CARGO_TOML_PATH not found. Run this script from the project root."

git rev-parse --show-toplevel >/dev/null 2>&1 || die "Not inside a Git repository"

if ! gh auth status &>/dev/null; then
    die "gh CLI not logged in. Run 'gh auth login' first."
fi

CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "$MAIN_BRANCH" ]; then
    die "You must be on the '$MAIN_BRANCH' branch. Current: ${CURRENT_BRANCH:-detached HEAD}"
fi

if [ -n "$(git status --porcelain)" ]; then
    die "Working directory is not clean. Please commit or stash changes first."
fi

echo "   Fetching $REMOTE..."
git fetch "$REMOTE" --tags --quiet

LOCAL_SHA=$(git rev-parse HEAD)
REMOTE_SHA=$(git rev-parse "$REMOTE/$MAIN_BRANCH" 2>/dev/null || true)
if [ -z "$REMOTE_SHA" ]; then
    die "Remote tracking branch '$REMOTE/$MAIN_BRANCH' not found. Does '$REMOTE' exist?"
fi
if [ "$LOCAL_SHA" != "$REMOTE_SHA" ]; then
    die "Local '$MAIN_BRANCH' must exactly match '$REMOTE/$MAIN_BRANCH' before reverting a release."
fi

if ! git show-ref --verify --quiet "refs/tags/$TAG"; then
    die "Tag '$TAG' does not exist locally"
fi
if ! git ls-remote --exit-code --tags "$REMOTE" "refs/tags/$TAG" >/dev/null 2>&1; then
    die "Tag '$TAG' does not exist on '$REMOTE'"
fi
if ! gh release view "$TAG" >/dev/null 2>&1; then
    die "GitHub release '$TAG' does not exist"
fi

RELEASE_COMMIT=$(git rev-list -n 1 "$TAG")
if [ "$RELEASE_COMMIT" != "$LOCAL_SHA" ]; then
    die "Release commit $RELEASE_COMMIT is not the current tip of '$MAIN_BRANCH'. Refusing to revert an older release automatically."
fi

PARENT_LINE=$(git rev-list --parents -n 1 "$RELEASE_COMMIT")
read -r -a COMMIT_AND_PARENTS <<< "$PARENT_LINE"
if [ "${#COMMIT_AND_PARENTS[@]}" -ne 2 ]; then
    die "Release commit must have exactly one parent"
fi
PREVIOUS_COMMIT="${COMMIT_AND_PARENTS[1]}"

RELEASE_SUBJECT=$(git log -1 --format=%s "$RELEASE_COMMIT")
if [ "$RELEASE_SUBJECT" != "$EXPECTED_SUBJECT" ]; then
    die "Tagged commit subject is '$RELEASE_SUBJECT', expected '$EXPECTED_SUBJECT'"
fi

VERSION_MANIFEST="$CARGO_TOML_PATH"
if grep -Eq '^[[:space:]]*version[[:space:]]*\.workspace[[:space:]]*=[[:space:]]*true' "$CARGO_TOML_PATH"; then
    VERSION_MANIFEST="$WORKSPACE_CARGO"
fi

RELEASE_VERSION=$(read_version_from_commit "$RELEASE_COMMIT" "$VERSION_MANIFEST") \
    || die "Could not read the version from $VERSION_MANIFEST at the release commit"
PREVIOUS_VERSION=$(read_version_from_commit "$PREVIOUS_COMMIT" "$VERSION_MANIFEST") \
    || die "Could not read the previous version from $VERSION_MANIFEST"

if [ "$RELEASE_VERSION" != "$VERSION" ]; then
    die "Release commit contains version '$RELEASE_VERSION', not requested version '$VERSION'"
fi
if [ "$PREVIOUS_VERSION" = "$VERSION" ]; then
    die "The parent commit already contains version '$VERSION'; there is no version bump to revert"
fi

RUN_SUMMARY=$(gh run list \
    --workflow "$RELEASE_WORKFLOW" \
    --branch "$TAG" \
    --event release \
    --limit 100 \
    --json databaseId,status,conclusion,url \
    --template '{{range .}}{{printf "   Run %v: status=%s conclusion=%s %s\\n" .databaseId .status .conclusion .url}}{{end}}')

ACTIVE_RUN_IDS=()
while IFS= read -r run_id; do
    [ -n "$run_id" ] && ACTIVE_RUN_IDS+=("$run_id")
done < <(gh run list \
    --workflow "$RELEASE_WORKFLOW" \
    --branch "$TAG" \
    --event release \
    --limit 100 \
    --json databaseId,status \
    --jq '.[] | select(.status == "queued" or .status == "in_progress" or .status == "waiting" or .status == "requested" or .status == "pending") | .databaseId')

echo "$(green "✓") All pre-flight checks passed"
echo ""
echo "   Branch:          $MAIN_BRANCH"
echo "   Release:         $TAG"
echo "   Release commit:  $RELEASE_COMMIT"
echo "   Version:         $VERSION → $PREVIOUS_VERSION"
echo "   Revert strategy: create a new commit; do not rewrite history"
echo "   Workflow runs:"
if [ -n "$RUN_SUMMARY" ]; then
    printf '%s' "$RUN_SUMMARY"
else
    echo "   No matching release workflow runs found"
fi

yellow "   ⚠ The release workflow may have published Docker tags to GHCR."
yellow "     This script will not delete or retag container images automatically."

# -------------------------------------------------------------------
# Confirmation / dry-run stop
# -------------------------------------------------------------------
if $DRY_RUN; then
    echo ""
    green "✔ Dry-run complete – no changes made."
    exit 0
fi

if ! $SKIP_CONFIRM; then
    echo ""
    yellow "This will cancel active release runs, delete the GitHub release and tag,"
    yellow "and push a revert commit to $REMOTE/$MAIN_BRANCH."
    read -r -p "$(yellow "?") Revert release $TAG? [y/N] " CONFIRM
    case "$CONFIRM" in
        [yY]|[yY][eE][sS]) ;;
        *) die "Aborted by user" ;;
    esac
fi

MUTATION_STARTED=true

# -------------------------------------------------------------------
# Cancel active release workflows
# -------------------------------------------------------------------
if [ "${#ACTIVE_RUN_IDS[@]}" -gt 0 ]; then
    STAGE="cancelling release workflow runs"
    echo "$(yellow "→") Cancelling active release workflow runs"

    for run_id in "${ACTIVE_RUN_IDS[@]}"; do
        status=$(gh run view "$run_id" --json status --jq '.status')
        if is_active_run_status "$status"; then
            echo "   Cancelling run $run_id ($status)"
            if ! gh run cancel "$run_id"; then
                status=$(gh run view "$run_id" --json status --jq '.status')
                is_active_run_status "$status" \
                    && die "Could not cancel active workflow run $run_id"
            fi
        fi
    done

    echo "   Waiting up to ${CANCEL_WAIT_SECONDS}s for cancellation to take effect..."
    deadline=$((SECONDS + CANCEL_WAIT_SECONDS))
    for run_id in "${ACTIVE_RUN_IDS[@]}"; do
        while true; do
            status=$(gh run view "$run_id" --json status --jq '.status')
            if ! is_active_run_status "$status"; then
                break
            fi
            if [ "$SECONDS" -ge "$deadline" ]; then
                die "Workflow run $run_id is still '$status'. Retry after cancellation completes."
            fi
            sleep 2
        done
        echo "   Run $run_id is no longer active"
    done

    echo "$(green "✓") Active release workflows stopped"
else
    echo "$(green "✓") No active release workflows to cancel"
fi

# -------------------------------------------------------------------
# Remove release and tags
# -------------------------------------------------------------------
STAGE="deleting GitHub release $TAG"
echo "$(yellow "→") Deleting GitHub release $TAG and its assets"
gh release delete "$TAG" --yes
echo "$(green "✓") GitHub release deleted"

STAGE="deleting remote tag $TAG"
echo "$(yellow "→") Deleting tag $TAG from $REMOTE"
git push "$REMOTE" ":refs/tags/$TAG"
echo "$(green "✓") Remote tag deleted"

STAGE="deleting local tag $TAG"
echo "$(yellow "→") Deleting local tag $TAG"
git tag -d "$TAG"
echo "$(green "✓") Local tag deleted"

# -------------------------------------------------------------------
# Revert version bump and push
# -------------------------------------------------------------------
STAGE="reverting release commit $RELEASE_COMMIT"
echo "$(yellow "→") Reverting release commit $RELEASE_COMMIT"
git revert --no-edit "$RELEASE_COMMIT"
REVERT_COMMIT=$(git rev-parse HEAD)
echo "$(green "✓") Created revert commit $REVERT_COMMIT"

STAGE="pushing revert commit to $REMOTE/$MAIN_BRANCH"
echo "$(yellow "→") Pushing revert commit to $REMOTE/$MAIN_BRANCH"
git push "$REMOTE" "$MAIN_BRANCH"
echo "$(green "✓") Revert commit pushed"

# -------------------------------------------------------------------
# Post-operation verification
# -------------------------------------------------------------------
STAGE="post-operation verification"
echo "$(yellow "→") Verifying rollback"

if gh release view "$TAG" >/dev/null 2>&1; then
    die "GitHub release '$TAG' still exists"
fi
if git ls-remote --exit-code --tags "$REMOTE" "refs/tags/$TAG" >/dev/null 2>&1; then
    die "Remote tag '$TAG' still exists"
fi
if git show-ref --verify --quiet "refs/tags/$TAG"; then
    die "Local tag '$TAG' still exists"
fi

POST_LOCAL_SHA=$(git rev-parse HEAD)
POST_REMOTE_SHA=$(git ls-remote --heads "$REMOTE" "refs/heads/$MAIN_BRANCH" | awk '{print $1}')
if [ -z "$POST_REMOTE_SHA" ] || [ "$POST_LOCAL_SHA" != "$POST_REMOTE_SHA" ]; then
    die "Local '$MAIN_BRANCH' does not match '$REMOTE/$MAIN_BRANCH' after push"
fi

RESTORED_VERSION=$(read_version_from_commit HEAD "$VERSION_MANIFEST") \
    || die "Could not verify restored version in $VERSION_MANIFEST"
if [ "$RESTORED_VERSION" != "$PREVIOUS_VERSION" ]; then
    die "Restored version is '$RESTORED_VERSION', expected '$PREVIOUS_VERSION'"
fi
if [ -n "$(git status --porcelain)" ]; then
    die "Working directory is not clean after rollback"
fi

trap - ERR
STAGE="complete"
echo ""
echo "$(green "✔") Release $TAG reverted successfully."
echo "   Version restored: $PREVIOUS_VERSION"
echo "   Revert commit:     $REVERT_COMMIT"
yellow "   Review GHCR package tags manually if the Docker release job published images."
