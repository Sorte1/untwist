# Releasing

A release puts the same version on crates.io, npm and https://untwist.dev. It starts from a
`vX.Y.Z` tag; nothing else deploys the site to production.

## Steps

```sh
cargo xtask bump X.Y.Z
git commit -am "chore: release X.Y.Z"
git tag vX.Y.Z
git push --follow-tags
```

`cargo xtask bump` sets the version in `Cargo.toml`, the internal version pins in
`crates/*/Cargo.toml`, `Cargo.lock`, `crates/untwist-wasm/npm/package.json`,
`web/package.json` and `web/package-lock.json`, and renames the CHANGELOG's Unreleased
section to `[X.Y.Z] - <today>` with a new empty Unreleased section above it. It refuses a
version that is not `MAJOR.MINOR.PATCH` or not newer than the current one. Edit the
CHANGELOG entry before committing if it needs it.

The tag runs `.github/workflows/release.yml`:

1. **verify** checks the tag with `cargo xtask check-version vX.Y.Z`, then runs fmt, clippy
   and the workspace tests.
2. **crates** publishes every publishable crate with `cargo publish --workspace`.
3. **npm** builds the wasm package and publishes `untwist` with provenance.
4. **web** deploys the site to production.
5. **github-release** creates the GitHub release with the CHANGELOG section as its notes.

The publish jobs skip a version that is already published, so a failed run can be rerun.

## One-time setup

Publishing uses trusted publishing (OIDC); the repository stores no crates.io or npm token.

- **crates.io.** For each published crate (`untwist`, `untwist-cli`, `untwist-core`,
  `untwist-deobfuscate`, `untwist-eval`, `untwist-jsx`, `untwist-rename`, `untwist-unminify`,
  `untwist-unpack`, `untwist-regress`): the crate's page › Settings › Trusted Publishing ›
  Add, with repository owner `Sorte1`, repository `untwist` and workflow `release.yml`.
- **npm.** On npmjs.com, package `untwist` › Settings › Trusted Publisher › GitHub Actions,
  with organization or user `Sorte1`, repository `untwist` and workflow filename `release.yml`.
- **Cloudflare.** The `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` repository secrets,
  and the Pages project `untwist` with production branch `main` (see `web/DEPLOY.md`).
