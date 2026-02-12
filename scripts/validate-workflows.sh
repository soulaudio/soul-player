#!/usr/bin/env bash
# Validate GitHub Actions workflows for syntax and common issues
#
# Usage:
#   ./scripts/validate-workflows.sh                    # Validate all workflows
#   ./scripts/validate-workflows.sh audio-e2e-tests    # Validate specific workflow

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if running from repo root
check_repo_root() {
    if [ ! -d ".github/workflows" ]; then
        log_error "Must be run from repository root"
        log_info "Current directory: $(pwd)"
        exit 1
    fi
}

# Validate YAML syntax using Python
validate_yaml_syntax() {
    local file=$1

    log_info "Validating YAML syntax: $file"

    if command -v python3 &> /dev/null; then
        python3 -c "
import yaml
import sys

try:
    with open('$file', 'r') as f:
        yaml.safe_load(f)
    print('✅ YAML syntax valid')
    sys.exit(0)
except yaml.YAMLError as e:
    print(f'❌ YAML syntax error: {e}')
    sys.exit(1)
except Exception as e:
    print(f'❌ Error: {e}')
    sys.exit(1)
"
        return $?
    else
        log_warn "Python3 not available, skipping YAML validation"
        return 0
    fi
}

# Check for common workflow issues
check_workflow_issues() {
    local file=$1
    local issues=0

    log_info "Checking for common issues: $file"

    # Check for hardcoded credentials
    if grep -q "password:" "$file" || grep -q "token:" "$file"; then
        if ! grep -q "secrets\." "$file"; then
            log_warn "Found hardcoded credentials (should use secrets)"
            issues=$((issues + 1))
        fi
    fi

    # Check for missing timeout
    if ! grep -q "timeout-minutes:" "$file"; then
        log_warn "No timeout-minutes defined (recommended for CI stability)"
        issues=$((issues + 1))
    fi

    # Check for uses: without version
    if grep -E "uses: [^@]+$" "$file" &> /dev/null; then
        log_warn "Found actions without version pinning (should use @v4, @main, etc.)"
        issues=$((issues + 1))
    fi

    # Check for continue-on-error without comment
    if grep -q "continue-on-error: true" "$file"; then
        log_warn "Found continue-on-error: true (ensure this is intentional)"
    fi

    # Check for shell without explicit type
    if grep -q "run:" "$file" && ! grep -q "shell:" "$file"; then
        log_info "Using default shell (consider explicit shell: bash/pwsh)"
    fi

    if [ $issues -eq 0 ]; then
        log_info "✅ No critical issues found"
        return 0
    else
        log_warn "⚠️  Found $issues potential issues"
        return 0
    fi
}

# Validate workflow naming and structure
check_workflow_structure() {
    local file=$1

    log_info "Checking workflow structure: $file"

    # Check for required fields
    local required_fields=("name" "on" "jobs")
    local missing=0

    for field in "${required_fields[@]}"; do
        if ! grep -q "^$field:" "$file"; then
            log_error "Missing required field: $field"
            missing=$((missing + 1))
        fi
    done

    if [ $missing -gt 0 ]; then
        log_error "Workflow missing $missing required fields"
        return 1
    fi

    # Check for job names
    if ! grep -q "^  [a-z-]*:" "$file"; then
        log_warn "No jobs found in workflow"
        return 1
    fi

    log_info "✅ Workflow structure valid"
    return 0
}

# Check workflow dependencies
check_workflow_dependencies() {
    local file=$1

    log_info "Checking workflow dependencies: $file"

    # Extract job names
    local jobs=$(grep -E "^  [a-z][a-z0-9-]*:" "$file" | sed 's/://g' | awk '{print $1}')

    # Check if needs: references exist
    for job in $jobs; do
        # Get needs for this job
        local needs=$(awk "/^  $job:/,/^  [a-z]/" "$file" | grep "needs:" | sed 's/needs://g' | tr -d '[],' | xargs)

        if [ -n "$needs" ]; then
            for need in $needs; do
                if ! echo "$jobs" | grep -q "^$need$"; then
                    log_error "Job '$job' depends on non-existent job '$need'"
                    return 1
                fi
            done
        fi
    done

    log_info "✅ Job dependencies valid"
    return 0
}

# Validate specific workflow features
validate_audio_e2e_workflow() {
    local file=".github/workflows/audio-e2e-tests.yml"

    log_info "=== Validating Audio E2E Workflow ==="

    # Check for platform-specific jobs
    local required_jobs=("audio-e2e-linux" "audio-e2e-macos" "audio-e2e-windows" "aggregate-results")

    for job in "${required_jobs[@]}"; do
        if ! grep -q "^  $job:" "$file"; then
            log_error "Missing required job: $job"
            return 1
        fi
    done

    log_info "✅ All required jobs present"

    # Check for virtual device setup in each platform
    if ! grep -q "snd-aloop" "$file"; then
        log_error "Missing Linux virtual device setup (snd-aloop)"
        return 1
    fi

    if ! grep -q "blackhole" "$file"; then
        log_error "Missing macOS virtual device setup (BlackHole)"
        return 1
    fi

    if ! grep -q "VBCABLE" "$file"; then
        log_error "Missing Windows virtual device setup (VB-Cable)"
        return 1
    fi

    log_info "✅ All platform virtual devices configured"

    # Check for retry logic
    if ! grep -q "run_with_retry" "$file" && ! grep -q "Run-WithRetry" "$file"; then
        log_warn "No retry logic found (recommended for E2E tests)"
    else
        log_info "✅ Retry logic present"
    fi

    # Check for metrics collection
    if ! grep -q "test-results" "$file"; then
        log_warn "No test metrics collection found"
    else
        log_info "✅ Metrics collection configured"
    fi

    # Check for artifact upload
    if ! grep -q "upload-artifact" "$file"; then
        log_warn "No artifact upload configured"
    else
        log_info "✅ Artifact upload configured"
    fi

    log_info "=== Audio E2E Workflow Validation Complete ==="
    return 0
}

# Main validation function
validate_workflow() {
    local file=$1
    local filename=$(basename "$file")

    echo ""
    log_info "================================================"
    log_info "Validating: $filename"
    log_info "================================================"

    # Run all validation checks
    validate_yaml_syntax "$file" || return 1
    check_workflow_structure "$file" || return 1
    check_workflow_issues "$file" || true  # Warnings only
    check_workflow_dependencies "$file" || return 1

    # Special validation for audio-e2e workflow
    if [[ "$filename" == "audio-e2e-tests.yml" ]]; then
        validate_audio_e2e_workflow || return 1
    fi

    log_info "✅ Validation passed: $filename"
    return 0
}

# Main script
main() {
    check_repo_root

    log_info "=== GitHub Actions Workflow Validator ==="
    log_info ""

    local specific_workflow="${1:-}"
    local failed=0

    if [ -n "$specific_workflow" ]; then
        # Validate specific workflow
        local workflow_file=".github/workflows/${specific_workflow}.yml"

        if [ ! -f "$workflow_file" ]; then
            log_error "Workflow file not found: $workflow_file"
            exit 1
        fi

        validate_workflow "$workflow_file" || failed=1
    else
        # Validate all workflows
        log_info "Validating all workflows in .github/workflows/"
        log_info ""

        for workflow in .github/workflows/*.yml .github/workflows/*.yaml; do
            if [ -f "$workflow" ]; then
                validate_workflow "$workflow" || failed=$((failed + 1))
            fi
        done
    fi

    echo ""
    log_info "================================================"

    if [ $failed -eq 0 ]; then
        log_info "✅ All validations passed!"
        exit 0
    else
        log_error "❌ $failed workflow(s) failed validation"
        exit 1
    fi
}

# Run main
main "$@"
