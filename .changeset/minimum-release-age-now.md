---
"pacquet": minor
---

Added the `minimumReleaseAgeNow` setting. `minimumReleaseAge` and `trustPolicyIgnoreAfter` count back from this RFC 3339 time, both when pnpm resolves versions and when it verifies a lockfile. Set it in `pnpm-workspace.yaml` or with `PNPM_CONFIG_MINIMUM_RELEASE_AGE_NOW`.
