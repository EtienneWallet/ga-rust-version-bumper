# ga-rust-version-bumper

A GitHub Action for custom target-based SemVer bumping in Rust workspaces, using Conventional Commits. Supports pre-release dev counters and stable promotions.

## Target-Based Version Bumping Philosophy

This action follows a deliberate, non-inflationary approach to versioning during active development. Instead of bumping the stable major/minor/patch digits with every commit or PR, we treat the base version (e.g. `1.3.0`) as the target stable release the team is currently working toward. The `-devN` suffix acts purely as a progress counter — incrementing with each fix, refactor, chore, doc change, or even small features that still fit within the planned scope of that target. Only when a change justifies shifting the target itself (e.g. a significant new feature when already at `x.y.z`, or a breaking change/refactor when not at `x.0.0`) do we reset lower digits and advance the appropriate part of the SemVer triplet. On `main`, the dev suffix is simply stripped to produce the final stable release.

## Prerequisites

> **Required in your consuming workflow:**
>
> ```yaml
> permissions:
>   contents: write
>
> steps:
>   - uses: actions/checkout@v4
>     with:
>       fetch-depth: 0   # full history required for tag reads
> ```

## Quick Start

```yaml
name: Version Bump

on:
  push:
    branches: [develop, main]

permissions:
  contents: write

jobs:
  bump:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Bump version
        id: bumper
        uses: EtienneWallet/ga-rust-version-bumper@main
        with:
          branch: ${{ github.ref_name }}

      - name: Use outputs
        run: echo "New version is ${{ steps.bumper.outputs.new-version }}"
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `branch` | yes | — | The branch being processed (e.g. `develop`, `main`) |
| `dry-run` | no | `false` | Log intended actions without committing, tagging, or pushing |
| `dev-branch` | no | `develop` | Branch to advance after a stable release on `main` |
| `next-dev-target` | no | `patch` | How to compute the next dev version: `patch` or `minor` |

## Outputs

| Output | Description |
|--------|-------------|
| `new-version` | Version string after bumping; empty if no-op |
| `previous-version` | Version string before any change |
| `bumped` | `true` if a version change was committed/tagged; `false` for skips and no-ops |
| `next-dev-version` | Version set on dev branch after stable release; empty on non-main |
| `dev-advance-failed` | `true` if stable release succeeded but dev branch push failed |

## Bump Rules (priority order)

| Condition | Commit type | Result |
|-----------|-------------|--------|
| Latest commit is an automated version commit | any | **skip** (no-op) |
| Branch is `main` | any | strip `-devN` → stable release, advance dev branch |
| `refactor` (with or without `!`) or breaking (`!` / `BREAKING CHANGE:`) with `major > 0` | — | major bump (unless at `x.0.0`, then dev bump) |
| `refactor` / breaking with `major == 0` | — | downgraded to minor: minor bump if `patch > 0`, else dev bump |
| `feat` | — | minor bump if `patch > 0`, else dev bump |
| everything else (`fix`, `docs`, `chore`, `style`, `perf`, `test`, …) | — | dev bump |

**Dev bump rule:** version with no `-devN` suffix → append `-dev0`. Version with `-devN` → increment to `-dev{N+1}`.

## Commit Type Examples

| Commit | Current | New |
|--------|---------|-----|
| `fix: handle null` | `1.2.3-dev5` | `1.2.3-dev6` |
| `feat: add OAuth` | `1.2.3-dev5` | `1.3.0-dev0` |
| `refactor: rework db` | `1.5.2-dev1` | `2.0.0-dev0` |
| `feat: anything` | `0.1.3-dev2` | `0.2.0-dev0` (minor bump, patch > 0) |
| `refactor: anything` | `1.0.0-dev5` | `1.0.0-dev6` (already at x.0.0) |
| `refactor: anything` | `0.5.0-dev3` | `0.5.0-dev4` (major=0 → minor, but patch=0 → dev) |
| `fix: anything` | `0.1.0` (no suffix) | `0.1.0-dev0` |

See `examples/commit-messages.md` for the full table.

## main Branch Flow

When `branch: main` is set:

1. Reads version from `Cargo.toml`; if already stable, exits zero (no-op).
2. Strips `-devN`: `1.3.0-dev10` → `1.3.0`.
3. Commits `chore: release stable 1.3.0`, tags `v1.3.0`, pushes to `main`.
4. Checks out `dev-branch` (default: `develop`).
5. Sets next dev version (e.g. `1.3.1-dev0`), commits `chore: start next development cycle 1.3.1-dev0`, pushes.
6. If dev branch push fails: logs a warning with manual remediation steps, sets `dev-advance-failed=true`, exits zero. The stable release is **not** rolled back.

## Cargo.toml Requirements

The action reads and writes the `version` field from:
- `[workspace.package]` (checked first)
- `[package]` (fallback)

See `examples/Cargo.toml.example` for supported formats.

## Troubleshooting

**Permission denied on push:**
Ensure your workflow has `permissions: contents: write` at the job or workflow level, and that `GITHUB_TOKEN` is available.

**Action loops / infinite triggers:**
The action automatically skips commits starting with `chore: bump version to`, `chore: release stable`, or `chore: start next development cycle`. If you see unexpected loops, check that your commit messages match exactly.

**Dev branch push fails after stable release:**
This is a best-effort operation. If `develop` diverged between the merge and the advancement attempt, the action logs the failure and exits zero. Manually set the version on `develop`:
```bash
git checkout develop
# Edit Cargo.toml version to the value shown in 'next-dev-version' output
git commit -m "chore: start next development cycle <version>"
git push
```

**Non-conventional commit messages:**
Commits that don't match `type[(scope)][!]: description` default to a dev bump with a warning logged. This is safe — no work is lost.
