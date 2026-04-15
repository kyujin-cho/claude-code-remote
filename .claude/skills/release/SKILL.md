---
name: release
description: Commit all changes, push to main, create a GitHub release with tag, watch CI/release workflows, and fix any failures automatically. Invoke with a version number argument.
argument-hint: <version e.g. v1.1.0>
disable-model-invocation: true
allowed-tools: Bash, Read, Edit, Write, Glob, Grep, Agent
---

# Release Workflow

Release version: `$ARGUMENTS`

Execute these steps in strict order. Do NOT skip steps. If a step fails, fix the issue and retry that step before proceeding.

## Step 1: Pre-flight Checks

1. Validate the version argument looks like `v*.*.*` (e.g. `v1.1.0`). If the user forgot the `v` prefix, add it.
2. Run `cargo fmt` to ensure formatting is clean.
3. Run `cargo clippy --all-targets --all-features -- -D warnings` to catch lint issues.
4. Run `cargo test --all-features` to verify tests pass.
5. If any of steps 2-4 fail, fix the issues, then re-run the failed check to confirm the fix before continuing.

## Step 2: Update Version

1. Extract the semver portion (strip the `v` prefix) from `$ARGUMENTS`.
2. Update `version = "..."` in `Cargo.toml` to the new version.
3. Run `cargo check` to ensure `Cargo.lock` is updated.

## Step 3: Update Documentation

Review and update if needed:
- `README.md` — version references, new features, changed commands
- `CLAUDE.md` — if architecture, commands, or features changed
- Check git diff to understand what changed since the last release to inform documentation updates

## Step 4: Commit and Push

1. Run `git status` to see all changes.
2. Stage all relevant files (be specific, don't use `git add -A`).
3. Create a commit with message: `chore: release $ARGUMENTS`
4. Push to main: `git push origin main`
5. Wait for the CI workflow to trigger. Get the run ID with:
   ```
   gh run list --workflow=ci.yml --limit=1 --json databaseId,status --jq '.[0].databaseId'
   ```

## Step 5: Watch CI Workflow

1. Watch the CI run:
   ```
   gh run watch <run-id> --exit-status
   ```
2. If CI **passes**, proceed to Step 6.
3. If CI **fails**:
   a. Get failure details: `gh run view <run-id> --log-failed`
   b. Identify the failing step (fmt, clippy, test, or build).
   c. Fix the issue locally.
   d. Commit the fix: `fix: resolve CI failure in <description>`
   e. Push to main: `git push origin main`
   f. Get the new CI run ID and go back to watching (repeat Step 5).
   g. Maximum 3 retry attempts. If still failing after 3 tries, stop and report the issue.

## Step 6: Create Git Tag and GitHub Release

1. Create an annotated tag:
   ```
   git tag -a $ARGUMENTS -m "Release $ARGUMENTS"
   ```
2. Push the tag:
   ```
   git push origin $ARGUMENTS
   ```
3. The release workflow triggers automatically on tag push. Get the run ID:
   ```
   gh run list --workflow=release.yml --limit=1 --json databaseId,status --jq '.[0].databaseId'
   ```

## Step 7: Watch Release Workflow

1. Watch the release run:
   ```
   gh run watch <run-id> --exit-status
   ```
2. If the release workflow **passes**, proceed to Step 8.
3. If it **fails**:
   a. Get failure details: `gh run view <run-id> --log-failed`
   b. If the failure is in the build matrix (e.g. a platform-specific issue), fix the workflow or code.
   c. If fixable: commit fix, push, delete the tag (`git tag -d $ARGUMENTS && git push origin :refs/tags/$ARGUMENTS`), and restart from Step 6.
   d. Maximum 3 retry attempts.

## Step 8: Verify Release

1. Verify the GitHub release exists and has all expected assets:
   ```
   gh release view $ARGUMENTS
   ```
2. Confirm these 5 files are attached:
   - `claude-code-telegram-linux-x86_64`
   - `claude-code-telegram-linux-aarch64`
   - `claude-code-telegram-macos-aarch64`
   - `claude-code-telegram-macos-x86_64`
   - `SHA256SUMS`
3. Print a summary of the release.

## Important Notes

- The CI workflow (`.github/workflows/ci.yml`) runs: `cargo fmt --check`, `cargo clippy --all-targets --all-features`, `cargo test --all-features`
- The release workflow (`.github/workflows/release.yml`) builds 4 platform binaries (linux x86_64, linux aarch64, macos aarch64, macos x86_64) and uploads them with SHA256 checksums
- Never force-push to main
- Never skip hooks or use `--no-verify`
- Do not include AI attribution in commit messages
