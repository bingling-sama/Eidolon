#!/usr/bin/env bash
set -euo pipefail

# Release preparation script for Eidolon
# Usage:
#   pnpm release <version>
#   ./scripts/release.sh <version>
# Example:
#   pnpm release 0.2.0

if [ -z "${1:-}" ]; then
  echo "Error: Version argument missing."
  echo "Usage: pnpm release <version> (e.g. pnpm release 0.2.0)"
  exit 1
fi

RAW_VERSION="$1"
# Strip leading 'v' if provided
VERSION="${RAW_VERSION#v}"

# Validate semantic version format (e.g., 0.2.0, 0.2.0-alpha.1)
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Error: Invalid version format '$VERSION'."
  echo "Expected SemVer format, e.g. 0.2.0 or 0.2.0-alpha.1"
  exit 1
fi

TAG="v$VERSION"

echo "==> Preparing release $TAG..."

# 1. Check git status
if [ -n "$(git status --porcelain)" ]; then
  echo "Error: Working directory has uncommitted changes. Please commit or stash first."
  git status --short
  exit 1
fi

# Check branch
CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$CURRENT_BRANCH" != "main" ]; then
  echo "Warning: You are currently on branch '$CURRENT_BRANCH', not 'main'."
  read -r -p "Continue anyway? (y/N): " confirm
  if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# Check if tag exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Git tag '$TAG' already exists."
  exit 1
fi

# 2. Check git-cliff availability
if command -v git-cliff >/dev/null 2>&1; then
  CLIFF_CMD="git-cliff"
elif command -v npx >/dev/null 2>&1; then
  CLIFF_CMD="npx --yes git-cliff"
else
  echo "Error: Neither 'git-cliff' nor 'npx' found."
  echo "Please install git-cliff: cargo install git-cliff"
  exit 1
fi

echo "==> Updating version in Cargo.toml and package.json to $VERSION..."

# 3. Update Cargo.toml workspace.package version and crates/wasm/package.json
node -e '
const fs = require("fs");
const version = process.argv[1];

// Update root Cargo.toml [workspace.package] version
const cargoPath = "Cargo.toml";
let cargo = fs.readFileSync(cargoPath, "utf8");
const updatedCargo = cargo.replace(
  /(\[workspace\.package\][^]*?version\s*=\s*")[^"]+(")/m,
  `$1${version}$2`
);
if (cargo === updatedCargo) {
  console.error("Warning: Cargo.toml version was not modified (already up to date?)");
}
fs.writeFileSync(cargoPath, updatedCargo);

// Update crates/wasm/package.json
const wasmPkgPath = "crates/wasm/package.json";
if (fs.existsSync(wasmPkgPath)) {
  const pkg = JSON.parse(fs.readFileSync(wasmPkgPath, "utf8"));
  pkg.version = version;
  fs.writeFileSync(wasmPkgPath, JSON.stringify(pkg, null, 2) + "\n");
}
' "$VERSION"

# 4. Sync Cargo.lock
echo "==> Updating Cargo.lock..."
cargo check -p eidolon-cli --quiet >/dev/null 2>&1 || cargo metadata --format-version 1 >/dev/null

# 5. Generate CHANGELOG.md
echo "==> Generating CHANGELOG.md via $CLIFF_CMD..."
$CLIFF_CMD --tag "$TAG" --output CHANGELOG.md

# 6. Git commit and tag
echo "==> Creating release commit and git tag..."
git add Cargo.toml Cargo.lock crates/wasm/package.json CHANGELOG.md
git commit -m "chore(release): $TAG"
git tag -a "$TAG" -m "Release $TAG"

echo ""
echo "================================================================"
echo "  Release $TAG prepared successfully!"
echo "================================================================"
echo ""
echo "Next step: push the release commit and tag to trigger CI/CD:"
echo ""
echo "  git push origin $CURRENT_BRANCH --tags"
echo ""
echo "To undo this release before pushing:"
echo "  git tag -d $TAG"
echo "  git reset --hard HEAD~1"
echo ""
