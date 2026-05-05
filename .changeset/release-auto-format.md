---
"@fltsci/taurpc": patch
---

Auto-format the auto-generated `CHANGELOG.md` in the Release workflow.

`@changesets/changelog-github` writes code-block samples with double quotes and trailing semicolons, which don't pass `dprint check`. Every prior `chore(release)` merge commit landed an unformatted `CHANGELOG.md`, which made the next `CI` run on main fail `lint:format` even though the npm publish itself succeeded. The new "Sync formatting if necessary" step in `release.yml` re-runs `pnpm format` on the auto-generated branch and amends the version-bump commit if anything changed -- analogous to the existing "Sync lockfile if necessary" step.
