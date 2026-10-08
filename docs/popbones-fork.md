<!-- SPDX-FileCopyrightText: 2026 Popbones -->
<!-- SPDX-License-Identifier: EUPL-1.2 -->
# Popbones fork

The personal branch is `popbones/develop`; upstream remains
`https://github.com/eza-community/eza`, branch `main`.

## macOS hidden entries

Directory listings hide dotfiles, entries with `UF_HIDDEN`, and entries with
`kIsInvisible` in `com.apple.FinderInfo`. `-a` / `-A` (including combined short
options) reveal them. This also applies to recursive and tree listings. Named
paths remain accessible, and symlinks use their own visibility metadata even
when `-X` is used. Missing or unreadable visibility metadata does not hide files.

## Daily upstream maintenance

`Popbones upstream rebase and build` runs daily at 19:17 UTC and can also be
dispatched manually. Human pushes to the personal branch run validation without
publishing another rebase; the bot's already-validated pushes are skipped.
The default branch must be `popbones/develop` for the
schedule to run. Upstream's inherited scheduled workflows are disabled in this fork.

The workflow fetches upstream, rebases locally, tests conflict handling and eza
(including native macOS metadata), checks formatting and Clippy, and builds a
macOS release. A failed rebase is aborted and leaves the remote unchanged.
Tests and the build must pass before a scheduled/manual run updates the branch.
An atomic push with explicit expected-old-SHA leases preserves the previous head
at `popbones/previous` and prevents overwriting concurrent changes. Validated
binaries, commit IDs, platform information, and a SHA-256 checksum are uploaded
as workflow artifacts and retained for 14 days. This does not update local installs.

`UPSTREAM_SYNC_SSH_KEY` is a repository secret containing a write-enabled deploy
key restricted to this fork. SSH permits rebasing upstream workflow changes
without requiring a personal access token with access to other repositories.

## Failure emails

GitHub sends scheduled workflow notifications to the user who owns the schedule
(the user who creates/edits the cron or enables the workflow). Configure
[GitHub notifications](https://github.com/settings/notifications): under
**System → Actions**, enable **Email**, optionally **Only notify for failed
workflows**. This is an account preference and is not controlled by a workflow.
Rebase, test, and build failures all fail this workflow and include a run summary.

To test email delivery, dispatch the workflow with `test_failure_notification`
set to `true`. It deliberately fails before checkout, without changing a branch.
The schedule and notification test should be enabled/dispatched as `popbones`.
GitHub can disable public-repository schedules after 60 days without repository
activity; re-enable the workflow if that occurs.

## Install or update

```sh
cargo install --git https://github.com/popbones/eza.git \
  --branch popbones/develop --locked --force
```

This installs `eza` under Cargo's binary directory, normally `~/.cargo/bin`.
`eza --version` identifies the Popbones fork.

## Local verification

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 -B -m unittest discover -s tests -p 'test_upstream_sync.py' -v
```
