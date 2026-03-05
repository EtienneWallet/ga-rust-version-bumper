# Commit Messages and Expected Version Bumps

| Commit Message | Current Version | Expected New Version | Reason |
|---|---|---|---|
| `fix: handle null input` | `1.2.3-dev5` | `1.2.3-dev6` | Dev bump |
| `docs: update README` | `1.2.3-dev5` | `1.2.3-dev6` | Dev bump |
| `chore: update deps` | `1.2.3-dev5` | `1.2.3-dev6` | Dev bump |
| `feat: add OAuth` | `1.2.3-dev5` | `1.3.0-dev0` | Minor bump (patch > 0) |
| `feat: add OAuth` | `1.2.0-dev3` | `1.2.0-dev4` | Dev bump (patch == 0) |
| `feat: add OAuth` | `0.1.3-dev2` | `0.2.0-dev0` | Minor bump (patch > 0, even with major == 0) |
| `refactor: rework db layer` | `1.5.2-dev1` | `2.0.0-dev0` | Major bump |
| `refactor!: breaking API` | `1.5.2-dev1` | `2.0.0-dev0` | Major bump (refactor always major) |
| `feat!: remove legacy API` | `1.5.2-dev1` | `2.0.0-dev0` | Major bump (breaking) |
| `refactor: anything` | `1.0.0-dev5` | `1.0.0-dev6` | Dev bump (already at x.0.0) |
| `refactor: anything` | `0.5.2-dev3` | `0.6.0-dev0` | Major downgraded to minor bump (major == 0, patch > 0) |
| `fix: anything` | `0.1.0` | `0.1.0-dev0` | Dev bump (no dev suffix treated as dev=0) |
| `chore: bump version to 1.2.3-dev6` | any | *skipped* | Loop detection |
| `chore: release stable 1.3.0` | any | *skipped* | Loop detection |
| `chore: start next development cycle 1.3.1-dev0` | any | *skipped* | Loop detection |

## Breaking Change Detection

Both of these produce a `Breaking` impact:

- `feat!: remove old API` (bang suffix)
- Commit with footer: `BREAKING CHANGE: old endpoint removed`

## main Branch Behavior

On `main`, commit type is ignored. The action always:

1. Strips the `-devN` suffix: `1.3.0-dev10` → `1.3.0`
2. Tags and pushes `v1.3.0` to main
3. Advances develop: `1.3.0` → `1.3.1-dev0` (with `next-dev-target: patch`)
