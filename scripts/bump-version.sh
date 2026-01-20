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
# - All package.json files (root + applications)
# - Tauri tauri.conf.json
#

set -uo pipefail
# Note: Not using -e (errexit) to allow counting failures and continue

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
        # Note: tauri.conf.json has version at root level, not under .package
        local temp_file=$(mktemp)
        jq ".version = \"$new_version\"" "$file" > "$temp_file"
        mv "$temp_file" "$file"
    else
        # Fallback to sed - update the first "version" field
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "0,/\"version\":/s/\"version\": \"[^\"]*\"/\"version\": \"$new_version\"/" "$file"
        else
            sed -i "0,/\"version\":/s/\"version\": \"[^\"]*\"/\"version\": \"$new_version\"/" "$file"
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
        for file in "$PROJECT_ROOT/libraries"/*/Cargo.toml; do
            if [ -f "$file" ]; then
                if update_cargo_toml "$file" "$new_version"; then
                    ((files_updated++))
                else
                    ((files_failed++))
                fi
            fi
        done
    fi

    # Update all application Cargo.toml files (in src-tauri directories)
    if [ -d "$PROJECT_ROOT/applications" ]; then
        for app_dir in "$PROJECT_ROOT/applications"/*; do
            if [ -d "$app_dir/src-tauri" ] && [ -f "$app_dir/src-tauri/Cargo.toml" ]; then
                if update_cargo_toml "$app_dir/src-tauri/Cargo.toml" "$new_version"; then
                    ((files_updated++))
                else
                    ((files_failed++))
                fi
            fi
        done
    fi

    # Update all package.json files (root + all applications)
    echo ""
    print_info "Updating package.json files..."

    # Root package.json
    if [ -f "$PROJECT_ROOT/package.json" ]; then
        if update_package_json "$PROJECT_ROOT/package.json" "$new_version"; then
            ((files_updated++))
        else
            ((files_failed++))
        fi
    fi

    # All application package.json files (one level deep)
    if [ -d "$PROJECT_ROOT/applications" ]; then
        for file in "$PROJECT_ROOT/applications"/*/package.json; do
            if [ -f "$file" ]; then
                if update_package_json "$file" "$new_version"; then
                    ((files_updated++))
                else
                    ((files_failed++))
                fi
            fi
        done
    fi

    echo ""

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
    print_info "Validating version updates..."
    echo ""

    # Verify tauri.conf.json version (CRITICAL)
    local tauri_version=""
    if command -v jq &> /dev/null; then
        tauri_version=$(jq -r '.version' "$tauri_conf")
    else
        # Fallback: use grep
        tauri_version=$(grep -m 1 '"version":' "$tauri_conf" | sed 's/.*"version": "\([^"]*\)".*/\1/')
    fi

    if [ "$tauri_version" != "$new_version" ]; then
        print_error "VALIDATION FAILED: tauri.conf.json version mismatch!"
        print_error "  Expected: $new_version"
        print_error "  Actual:   $tauri_version"
        print_warning "This will cause UI to show wrong version!"
        ((files_failed++))
    else
        print_success "Validation: tauri.conf.json version = $tauri_version ✓"
    fi

    # Verify workspace Cargo.toml version
    local cargo_version=$(grep -m 1 '^version = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/version = "\(.*\)"/\1/')
    if [ "$cargo_version" != "$new_version" ]; then
        print_error "VALIDATION FAILED: Cargo.toml version mismatch!"
        print_error "  Expected: $new_version"
        print_error "  Actual:   $cargo_version"
        ((files_failed++))
    else
        print_success "Validation: Cargo.toml version = $cargo_version ✓"
    fi

    echo ""
    echo "═══════════════════════════════════════════════════════"

    if [ $files_failed -eq 0 ]; then
        print_success "Version bump complete!"
        print_info "Updated $files_updated file(s)"
    else
        print_error "Version bump failed - cannot proceed with commit"
        print_warning "Failed to update $files_failed file(s)"
        exit 1
    fi

    echo ""
    print_info "Committing changes and creating tag..."
    echo ""

    # Stage all changes
    print_info "Staging all changes..."
    git add -A

    # Show what will be committed
    echo ""
    echo "=== Changes to commit ==="
    git status --short
    echo ""

    # Commit with conventional commit message
    local commit_message="chore: bump version to v$new_version

- Updated all Cargo.toml files to v$new_version
- Updated all package.json files to v$new_version
- Updated tauri.conf.json to v$new_version
- Includes previous fixes and improvements"

    print_info "Creating commit..."
    if git commit -m "$commit_message"; then
        print_success "Commit created successfully"
    else
        print_error "Failed to create commit"
        exit 1
    fi

    # Create and push tag
    local tag_name="v$new_version"
    print_info "Creating tag: $tag_name"
    if git tag -a "$tag_name" -m "Release $new_version"; then
        print_success "Tag created: $tag_name"
    else
        print_error "Failed to create tag"
        exit 1
    fi

    # Push commits and tags
    echo ""
    print_info "Pushing to origin..."
    if git push origin main && git push origin "$tag_name"; then
        print_success "Successfully pushed commits and tag!"
    else
        print_error "Failed to push to origin"
        print_warning "You may need to push manually:"
        echo "  git push origin main"
        echo "  git push origin $tag_name"
        exit 1
    fi

    echo ""
    echo "═══════════════════════════════════════════════════════"
    print_success "Release v$new_version initiated!"
    echo ""
    print_info "GitHub Actions will now:"
    echo "  • Detect the new tag v$new_version"
    echo "  • Trigger the release workflow"
    echo "  • Build installers for Windows, macOS, Linux"
    echo "  • Build Flatpak package"
    echo "  • Publish to AUR"
    echo "  • Create GitHub release"
    echo ""
    print_info "Monitor release progress at:"
    echo "  https://github.com/soulaudio/soul-player/actions"
    echo ""
    print_success "Script complete!"
}

# Check if running from project root or scripts directory
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    print_error "Could not find project root (Cargo.toml not found)"
    print_info "Please run this script from the project root or scripts directory"
    exit 1
fi

# Run main function
main "$@"
