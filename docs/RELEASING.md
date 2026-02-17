# Releasing

This document describes how to create a new release of my-open-claude.

## Prerequisites

- Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, etc.)
- Development happens on `feature/*` or `fix/*` branches; PRs are merged into `master`
- **Important**: When using squash-merge, the merge commit message must follow Conventional Commits so that release-please detects changes (e.g. `feat: add X` or `fix: correct Y`)

## Automatic release process

1. Merge PRs (from `feature/*` or `fix/*`) into `master`
2. release-please opens a Release PR when it detects `feat:` or `fix:` commits since the last tag
3. Review and merge the Release PR
4. release-please creates the version tag (e.g. `v0.1.1`)
5. The `release.yml` workflow builds binaries and creates the GitHub release with assets

## Forcing a specific version

To release a specific version, add `Release-As: X.Y.Z` in the body of a commit or PR description. For example:

```bash
git commit --allow-empty -m "chore: release 0.2.0" -m "Release-As: 0.2.0"
```

## First-time setup

Ensure `.release-please-manifest.json` contains the current version (it should match `Cargo.toml`). This file is updated by release-please after each release.

## Bootstrapping the first release

If no release tags exist yet, release-please may not open a Release PR immediately. To force the first release:

```bash
git commit --allow-empty -m "chore: release 0.1.0" -m "Release-As: 0.1.0"
git push origin master
```

Then merge the resulting Release PR when it appears.
