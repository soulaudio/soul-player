# Audio E2E CI/CD Deployment Checklist

Use this checklist to verify the complete implementation and deployment of the Audio E2E testing infrastructure.

## Pre-Deployment

### Repository Preparation

- [ ] Repository is on GitHub (workflow requires GitHub Actions)
- [ ] Branch protection rules allow workflow runs
- [ ] Actions are enabled in repository settings
- [ ] Secrets are configured (if using private runners)

### Local Testing

- [ ] Virtual audio device installed locally (Linux: snd-aloop, macOS: BlackHole, Windows: VB-Cable)
- [ ] Tests pass locally on your development platform
- [ ] Scripts are executable (`chmod +x scripts/*.sh`)
- [ ] Documentation reviewed and accurate

## File Verification

### Workflow Files

- [ ] `.github/workflows/audio-e2e-tests.yml` exists
- [ ] YAML syntax is valid (run `./scripts/validate-workflows.sh audio-e2e-tests`)
- [ ] All platform jobs are present (linux, macos, windows)
- [ ] Virtual device setup steps are correct for each platform
- [ ] Retry logic is implemented
- [ ] Metrics collection is configured
- [ ] Artifact upload is present
- [ ] PR comment job is configured

### Documentation Files

- [ ] `docs/AUDIO_E2E_TESTING.md` exists and is complete
- [ ] `docs/AUDIO_E2E_QUICK_START.md` exists
- [ ] `docs/README_BADGE_TEMPLATE.md` exists
- [ ] `AUDIO_E2E_CI_COMPLETE.md` exists (summary document)
- [ ] All documentation links are valid
- [ ] Code examples in docs are accurate

### Script Files

- [ ] `scripts/setup-virtual-audio.sh` exists
- [ ] `scripts/setup-virtual-audio.ps1` exists
- [ ] `scripts/validate-workflows.sh` exists
- [ ] All scripts are executable on Unix systems
- [ ] Scripts have been tested on target platforms

## Workflow Configuration

### Linux Job

- [ ] Uses `ubuntu-latest` runner
- [ ] Installs system dependencies (libasound2-dev, etc.)
- [ ] Loads snd-aloop kernel module
- [ ] Creates ALSA configuration
- [ ] Verifies device availability
- [ ] Runs tests with retry logic
- [ ] Collects metrics
- [ ] Uploads artifacts

### macOS Job

- [ ] Uses `macos-latest` runner
- [ ] Installs Homebrew dependencies
- [ ] Installs BlackHole via brew
- [ ] Waits for device initialization
- [ ] Verifies device availability
- [ ] Runs tests with retry logic
- [ ] Collects metrics
- [ ] Uploads artifacts

### Windows Job

- [ ] Uses `windows-latest` runner
- [ ] Installs LLVM for ASIO support
- [ ] Downloads VB-Cable
- [ ] Installs VB-Cable driver
- [ ] Waits for driver initialization
- [ ] Verifies device availability
- [ ] Runs tests with retry logic
- [ ] Collects metrics
- [ ] Uploads artifacts

### Aggregate Job

- [ ] Depends on all platform jobs (`needs: [...]`)
- [ ] Downloads all artifacts
- [ ] Parses metrics JSON files
- [ ] Generates GitHub Step Summary
- [ ] Creates combined metrics artifact
- [ ] Checks overall status

### PR Comment Job

- [ ] Only runs on pull requests (`if: github.event_name == 'pull_request'`)
- [ ] Has pull-requests: write permission
- [ ] Downloads all artifacts
- [ ] Parses metrics correctly
- [ ] Posts formatted comment
- [ ] Handles missing data gracefully

## Test Coverage

### Test Files

- [ ] `libraries/soul-audio-desktop/tests/device_hotplug_e2e.rs` exists
- [ ] `libraries/soul-audio-desktop/tests/playback_hotplug_integration_e2e.rs` exists
- [ ] `libraries/soul-audio-desktop/tests/device_handling_test.rs` exists
- [ ] Platform-specific tests exist (platform_linux_test.rs, etc.)
- [ ] All tests compile without errors
- [ ] Tests include proper error handling

### Test Execution

- [ ] Tests run with `--test-threads=1` (required for audio)
- [ ] Tests use `--release` mode in CI
- [ ] Tests output to both stdout and JSON
- [ ] Tests handle missing devices gracefully
- [ ] Tests include performance assertions

## Deployment Steps

### 1. Commit and Push

```bash
# Stage all new files
git add .github/workflows/audio-e2e-tests.yml
git add docs/AUDIO_E2E_*.md
git add docs/README_BADGE_TEMPLATE.md
git add scripts/setup-virtual-audio.*
git add scripts/validate-workflows.sh
git add AUDIO_E2E_CI_COMPLETE.md

# Commit
git commit -m "feat: add comprehensive audio E2E CI/CD workflows

- Add GitHub Actions workflow for Linux, macOS, Windows
- Virtual audio device setup for all platforms
- Retry logic for CI stability
- Metrics collection and artifact upload
- PR integration with automated comments
- Complete documentation and setup scripts"

# Push to main or develop
git push origin main
```

- [ ] Files committed to repository
- [ ] Pushed to GitHub
- [ ] Commit message is descriptive

### 2. Verify Workflow Recognition

```bash
# Check workflow is recognized
gh workflow list

# Expected output should include:
# Audio E2E Tests  active  <workflow-id>
```

- [ ] Workflow appears in `gh workflow list`
- [ ] Workflow file is in default branch
- [ ] No YAML syntax errors reported

### 3. Trigger Initial Run

```bash
# Manual trigger (if workflow_dispatch is configured)
gh workflow run audio-e2e-tests.yml

# Or create a PR to trigger automatically
```

- [ ] Workflow triggered successfully
- [ ] All jobs start
- [ ] No immediate failures

### 4. Monitor First Run

```bash
# Watch the run in real-time
gh run watch

# Or view in browser
gh run view --web
```

- [ ] Linux job starts and completes
- [ ] macOS job starts and completes
- [ ] Windows job starts and completes
- [ ] Aggregate job runs after platform jobs
- [ ] All jobs show green checkmarks

## Post-Deployment Verification

### Workflow Runs

- [ ] First run completed successfully on all platforms
- [ ] Retry logic works (check logs for retry attempts if any failures)
- [ ] Execution time is within expected range (~8-12 minutes)
- [ ] No timeout errors
- [ ] No out-of-memory errors

### Artifacts

```bash
# Download artifacts from first successful run
gh run list --workflow=audio-e2e-tests.yml --limit 1
gh run download <run-id> -n audio-e2e-metrics-combined
```

- [ ] Artifacts uploaded successfully
- [ ] Linux metrics file exists and is valid JSON
- [ ] macOS metrics file exists and is valid JSON
- [ ] Windows metrics file exists and is valid JSON
- [ ] Combined metrics artifact contains all platforms

### Metrics Quality

- [ ] Linux summary includes platform, runner, device, timestamp, result
- [ ] macOS summary includes platform, runner, device, timestamp, result
- [ ] Windows summary includes platform, runner, device, timestamp, result
- [ ] Timestamps are in ISO 8601 format
- [ ] Test results are "success" or "failure" (not undefined)

### PR Integration

- [ ] Create a test PR
- [ ] Workflow runs on PR creation
- [ ] Comment is posted to PR with results
- [ ] Comment includes all three platforms
- [ ] Comment formatting is correct
- [ ] Workflow link in comment works

### GitHub Step Summary

- [ ] Summary appears in workflow run UI
- [ ] Table includes all platforms
- [ ] Status badges are correct
- [ ] Timestamp is accurate
- [ ] Artifact links work

## Performance Validation

### Execution Time

- [ ] Linux job: < 10 minutes
- [ ] macOS job: < 10 minutes
- [ ] Windows job: < 15 minutes
- [ ] Total workflow: < 20 minutes

### Resource Usage

- [ ] No disk space errors
- [ ] No memory errors
- [ ] CPU usage reasonable (no infinite loops)

### Test Performance

Check metrics for:
- [ ] Device enumeration: Linux ~60ms, macOS ~30ms, Windows ~100ms
- [ ] Hotplug start: Linux ~100ms, macOS ~50ms, Windows ~150ms
- [ ] Full test suite: Linux ~30s, macOS ~25s, Windows ~40s

## Documentation Updates

### README.md

- [ ] Add Audio E2E Tests badge
- [ ] Add testing section referencing docs
- [ ] Update build status table if present
- [ ] Include quick start commands

### CHANGELOG.md (if present)

- [ ] Add entry for Audio E2E CI/CD implementation
- [ ] Note virtual device support for all platforms
- [ ] Mention automated PR comments

## Optional Enhancements

### Branch Protection

- [ ] Require Audio E2E tests to pass before merging
- [ ] Add workflow as required status check
- [ ] Configure in repository settings

### Notifications

- [ ] Configure Slack/Discord notifications for failures
- [ ] Set up email alerts for maintainers
- [ ] Add workflow status to project dashboard

### Integration with Main CI

- [ ] Add dependency in `.github/workflows/ci.yml`
- [ ] Update ci-success job to include audio-e2e
- [ ] Ensure proper job ordering

## Troubleshooting First Deployment

### Common Issues

**Issue:** Workflow doesn't appear in Actions tab
- [ ] Check YAML syntax with validator
- [ ] Ensure file is in `.github/workflows/`
- [ ] Verify file has `.yml` extension
- [ ] Push to default branch (usually `main`)

**Issue:** Job fails immediately
- [ ] Check workflow logs for error messages
- [ ] Verify runner availability (ubuntu-latest, macos-latest, windows-latest)
- [ ] Check for typos in job names or dependencies

**Issue:** Virtual device setup fails
- [ ] Normal for Windows VB-Cable (may need admin)
- [ ] Check if module/package exists in CI environment
- [ ] Verify fallback logic works (tests continue)

**Issue:** Tests timeout
- [ ] Increase timeout-minutes in job configuration
- [ ] Check if tests are actually running (not hung)
- [ ] Verify test parallelism is set to 1

**Issue:** Artifacts not uploaded
- [ ] Check if test-results directory was created
- [ ] Verify metrics collection step ran
- [ ] Check artifact upload step logs

**Issue:** PR comment not posted
- [ ] Verify job has pull-requests: write permission
- [ ] Check if workflow runs on pull_request events
- [ ] Look for errors in GitHub Script step

## Final Checklist

### Deployment Complete

- [ ] All workflow jobs passing
- [ ] All artifacts collecting properly
- [ ] PR comments working
- [ ] Documentation complete and accurate
- [ ] Scripts tested and working
- [ ] README updated with badge
- [ ] Team notified of new workflow

### Follow-Up Tasks

- [ ] Monitor first few runs for stability
- [ ] Collect feedback from team
- [ ] Update documentation based on real-world usage
- [ ] Consider adding to release requirements
- [ ] Plan for future enhancements (Docker, stress tests, etc.)

## Sign-Off

**Deployed By:** ___________________
**Date:** ___________________
**Workflow Version:** 1.0
**Platforms Verified:** Linux ☐ macOS ☐ Windows ☐

**Notes:**
_______________________________________________________________________
_______________________________________________________________________
_______________________________________________________________________

---

**Reference Documents:**
- Full Documentation: `docs/AUDIO_E2E_TESTING.md`
- Quick Start: `docs/AUDIO_E2E_QUICK_START.md`
- Implementation Summary: `AUDIO_E2E_CI_COMPLETE.md`
- Badge Template: `docs/README_BADGE_TEMPLATE.md`

**Support:**
- GitHub Issues: https://github.com/soulaudio/soul-player/issues
- Workflow Status: https://github.com/soulaudio/soul-player/actions/workflows/audio-e2e-tests.yml
