---
name: commit
description: Review, verify, version, commit, annotate and push current Koldron changes. Use whenever the user asks to commit, save changes to Git, checkpoint, or check in work. Every successful commit gets a new version and Git tag, but never a GitHub Release.
---

# Commit: version every change, release nothing

A successful invocation creates **one commit, one unique SemVer version, and one annotated Git tag** (`vX.Y.Z`). Push the commit and tag. Do **not** build installers or create a GitHub Release. Work locally except for the final Git push.

1. **Inspect before modifying anything.** Check the branch, `git status --short`, `git diff HEAD`, staged changes, and the contents of relevant untracked files. Stop if there are no changes. Avoid unrelated files, generated outputs, secrets, and ignored local data. Never use `git add -A` without reviewing what would be staged. Do not discard other work.
2. **Determine the change level.** Use `patch` for fixes, styling, refactors, tests, documentation, and maintenance; `minor` for backward-compatible new functionality; `major` for a breaking change to a supported public API/SDK. Follow any documented pre-1.0 policy in `CLAUDE.md`. If ambiguous, ask. Every commit bumps at least `patch`, even for docs-only changes. The magnitude of work alone never makes a change `major`.
3. **Check versions and collisions.** Read the version in `package.json`, verify the local/remote version tags and the versions in Tauri/Rust manifests. Fetch remote tags if available; do not overwrite/reuse existing tags. Stop on inconsistent version sources or branch/remote divergence, and report the conflict. Never force-push or force-update a tag.
4. **Prepare changelog entries.** Write short, factual, user-facing `Added`, `Changed`, and/or `Fixed` entries based on the actual diff. Use at least one entry. Do not invent features or duplicate entries from earlier versions. Update the README's status/setup only when behavior, setup, or availability materially changed.
5. **Bump once.** Prefer the repository's verified `node scripts/version.mjs bump <patch|minor|major> --added "..." --changed "..." --fixed "..."` command, repeating flags as needed. It must update `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, README's displayed version, and `CHANGELOG.md`. Check that this script and expected files actually exist; if not, stop and explain what is missing instead of fabricating or partially editing versions. On retry after a test failure, **do not bump again**.
6. **Review and verify locally.** Inspect the version/changelog diff and run `npm run verify` (or the documented equivalent). Let repository hooks run. Fix introduced failures, then rerun affected checks. Never use `--no-verify`. If verification cannot pass, leave the repository uncommitted and report the failure and existing version bump.
7. **Stage deliberately and commit.** Stage the intended source, version, docs and changelog files. Inspect `git diff --cached --stat` and `git diff --cached --check`, and review the staged diff for secrets and unrelated changes. Commit using `feat|fix|refactor|docs|test|chore: concise summary (vX.Y.Z)`; include co-author attribution only if available and appropriate.
8. **Tag only after commit success.** Confirm the new commit is `HEAD`; create `git tag -a vX.Y.Z -m "vX.Y.Z"`. Verify the tag points to `HEAD` and is unique.
9. **Push safely.** Push the current branch and the exact new tag to the configured remote, without force. If push fails, preserve the local commit/tag and report how to recover; do not create another version. Verify remote results when possible.
10. **Report succinctly.** State version, level and reason, commit hash, verification outcome, tag and push status. Explicitly say **not published as a GitHub Release**.

## Invariants
- A Git tag represents a versioned source checkpoint; only the separate `/publish` skill creates downloadable GitHub Releases.
- Work within the current repository; do not create branches, reset history, bypass hooks, or use GitHub Actions.
- Keep changelog headings in the format `## [X.Y.Z] - YYYY-MM-DD`, with `### Added`, `### Changed`, `### Fixed` sections as applicable, for later aggregation by `/publish`.
