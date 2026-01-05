#!/usr/bin/env bash
#
# Version Bumping Script for Soul Player
#
# Usage: ./scripts/bump-version.sh <version>
# Example: ./scripts/bump-version.sh 0.1.0
#
# This script updates version numbers in:
# - Workspace Cargo.toml
# - All crate Cargo.toml files
# - Tauri package.json
# - Tauri tauri.conf.json
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Function to print colored output
print_error() {
    echo -e "${RED}❌ $1${NC}" >&2
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Function to validate version format (semver)
validate_version() {
    local version="$1"

    # Check format: X.Y.Z or X.Y.Z-prerelease
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
        print_error "Invalid version format: $version"
        print_info "Expected format: X.Y.Z (e.g., 0.1.0)"
        print_info "Or with pre-release: X.Y.Z-alpha.1, X.Y.Z-beta.1, X.Y.Z-rc.1"
        return 1
    fi

    return 0
}

# Function to get current version from workspace Cargo.toml
get_current_version() {
    if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
        grep -m 1 '^version = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/version = "\(.*\)"/\1/'
    else
        echo "unknown"
    fi
}

# Function to update version in Cargo.toml file
update_cargo_toml() {
    local file="$1"
    local new_version="$2"

    if [ ! -f "$file" ]; then
        print_warning "File not found: $file"
        return 1
    fi

    # Update the first occurrence of version = "..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS sed syntax
        sed -i '' "1,/^version = /s/^version = \".*\"/version = \"$new_version\"/" "$file"
    else
        # Linux sed syntax
        sed -i "1,/^version = /s/^version = \".*\"/version = \"$new_version\"/" "$file"
    fi

    print_success "Updated: $file"
    return 0
}

# Function to update version in package.json
update_package_json() {
    local file="$1"
    local new_version="$2"

    if [ ! -f "$file" ]; then
        print_warning "File not found: $file"
        return 1
    fi

    # Check if jq is available
    if command -v jq &> /dev/null; then
        # Use jq for JSON manipulation (safer)
        local temp_file=$(mktemp)
        jq ".version = \"$new_version\"" "$file" > "$temp_file"
        mv "$temp_file" "$file"
    else
        # Fallback to sed
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/\"version\": \".*\"/\"version\": \"$new_version\"/" "$file"
        else
            sed -i "s/\"version\": \".*\"/\"version\": \"$new_version\"/" "$file"
        fi
    fi

    print_success "Updated: $file"
    return 0
}

# Function to update version in tauri.conf.json
update_tauri_conf() {
    local file="$1"
    local new_version="$2"

    if [ ! -f "$file" ]; then
        print_warning "File not found: $file"
        return 1
    fi

    # Check if jq is available
    if command -v jq &> /dev/null; then
        # Use jq for JSON manipulation (safer)
        local temp_file=$(mktemp)
        jq ".package.version = \"$new_version\"" "$file" > "$temp_file"
        mv "$temp_file" "$file"
    else
        # Fallback to sed (less reliable for nested JSON)
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/\"version\": \".*\"/\"version\": \"$new_version\"/" "$file"
        else
            sed -i "s/\"version\": \".*\"/\"version\": \"$new_version\"/" "$file"
        fi
    fi

    print_success "Updated: $file"
    return 0
}

# Function to find all Cargo.toml files
find_cargo_tomls() {
    find "$PROJECT_ROOT" -name "Cargo.toml" -type f \
        -not -path "*/target/*" \
        -not -path "*/node_modules/*" \
        -not -path "*/.git/*"
}

# Main function
main() {
    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Soul Player Version Bumping Script"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    # Check arguments
    if [ $# -ne 1 ]; then
        print_error "Usage: $0 <version>"
        echo ""
        echo "Examples:"
        echo "  $0 0.1.0"
        echo "  $0 0.2.0-beta.1"
        echo "  $0 1.0.0"
        exit 1
    fi

    local new_version="$1"

    # Validate version format
    if ! validate_version "$new_version"; then
        exit 1
    fi

    # Get current version
    local current_version=$(get_current_version)

    print_info "Current version: $current_version"
    print_info "New version:     $new_version"
    echo ""

    # Confirm with user
    read -p "Continue with version bump? [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_warning "Version bump cancelled"
        exit 0
    fi

    echo ""
    print_info "Updating version numbers..."
    echo ""

    local files_updated=0
    local files_failed=0

    # Update workspace Cargo.toml
    if update_cargo_toml "$PROJECT_ROOT/Cargo.toml" "$new_version"; then
        ((files_updated++))
    else
        ((files_failed++))
    fi

    # Update all library Cargo.toml files
    if [ -d "$PROJECT_ROOT/libraries" ]; then
        while IFS= read -r file; do
            if update_cargo_toml "$file" "$new_version"; then
                ((files_updated++))
            else
                ((files_failed++))
            fi
        done < <(find "$PROJECT_ROOT/libraries" -name "Cargo.toml" -type f)
    fi

    # Update all application Cargo.toml files
    if [ -d "$PROJECT_ROOT/applications" ]; then
        while IFS= read -r file; do
            # Skip package.json and tauri config (handled separately)
            if [[ "$file" == *"/Cargo.toml" ]]; then
                if update_cargo_toml "$file" "$new_version"; then
                    ((files_updated++))
                else
                    ((files_failed++))
                fi
            fi
        done < <(find "$PROJECT_ROOT/applications" -name "Cargo.toml" -type f)
    fi

    # Update Tauri package.json
    local tauri_package_json="$PROJECT_ROOT/applications/desktop/package.json"
    if [ -f "$tauri_package_json" ]; then
        if update_package_json "$tauri_package_json" "$new_version"; then
            ((files_updated++))
        else
            ((files_failed++))
        fi
    fi

    # Update Tauri config
    local tauri_conf="$PROJECT_ROOT/applications/desktop/src-tauri/tauri.conf.json"
    if [ -f "$tauri_conf" ]; then
        if update_tauri_conf "$tauri_conf" "$new_version"; then
            ((files_updated++))
        else
            ((files_failed++))
        fi
    fi

    echo ""
    echo "═══════════════════════════════════════════════════════"

    if [ $files_failed -eq 0 ]; then
        print_success "Version bump complete!"
        print_info "Updated $files_updated file(s)"
    else
        print_warning "Version bump completed with warnings"
        print_info "Updated $files_updated file(s)"
        print_warning "Failed to update $files_failed file(s)"
    fi

    echo ""
    print_info "Next steps:"
    echo "  1. Review changes: git diff"
    echo "  2. Run tests: cargo test --all"
    echo "  3. Commit changes: git commit -am 'chore: bump version to v$new_version'"
    echo "  4. Create tag: git tag -a v$new_version -m 'Release v$new_version'"
    echo "  5. Push: git push origin main && git push origin v$new_version"
    echo ""

    # Optionally show git diff
    read -p "Show git diff? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        git diff
    fi
}

# Check if running from project root or scripts directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    print_error "Could not find project root (Cargo.toml not found)"
    print_info "Please run this script from the project root or scripts directory"
    exit 1
fi

# Run main function
main "$@"
