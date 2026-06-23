# Releasing tmosh

Releases are cut by pushing a `vX.Y.Z` tag. The
[`Release`](.github/workflows/release.yml) workflow then cross-builds the
binaries, uploads them as build artifacts, and publishes them — plus
`SHA256SUMS` — as assets on the GitHub Release that the installer and
self-updater download from.

## Steps

1. **Bump the version** in [`Cargo.toml`](Cargo.toml) (`version = "X.Y.Z"`).
   The binary reports this via `--version`, and the self-updater compares
   against it, so it must match the tag.

2. **Update `Cargo.lock`** and run the checks locally:

   ```sh
   cargo build --release        # refreshes Cargo.lock
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```

3. **Commit** the bump:

   ```sh
   git commit -am "Release vX.Y.Z"
   git push origin main
   ```

4. **Tag and push** — this triggers the release:

   ```sh
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

   The workflow's first job verifies the tag equals the `Cargo.toml` version
   and fails fast if they differ — so a mismatched tag never wastes a build.

5. **Verify** the Release page lists all four assets
   (`tmosh-{x86_64,aarch64}-{unknown-linux-gnu,apple-darwin}`) and
   `SHA256SUMS`, then test the installer:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/totophe/tmosh/main/install.sh | sh
   ```

## If a tag was pushed wrong

```sh
git tag -d vX.Y.Z                 # delete locally
git push origin :refs/tags/vX.Y.Z # delete remotely
# fix Cargo.toml / commit, then re-tag and push again
```
