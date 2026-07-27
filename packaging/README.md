# Packaging

Distribution channels beyond `install.sh` / `install.ps1` / npm / crates.io.

Everything here is **inert until you create the corresponding public repository**
and add the secret named below. The release workflow skips each publish step when
its secret is absent, so tagging a release with none of this set up behaves
exactly as it does today.

## What exists

| File | Channel | Needs |
|---|---|---|
| `scoop/anesis.json` | Scoop (Windows) | repo `anesis-dev/scoop-anesis`, secret `PACKAGING_TOKEN` |
| `homebrew/anesis.rb` | Homebrew tap (macOS + Linux) | repo `anesis-dev/homebrew-anesis`, secret `PACKAGING_TOKEN` |

Both files are **templates**: `{{VERSION}}` and the `{{SHA256_*}}` placeholders are
substituted by `packaging/render.sh` from the release's `SHA256SUMS`, and the
result is pushed to the tap/bucket repository. They are checked in so the shape
is reviewable, and so the substitution can be tested without cutting a release.

## Setting it up

1. Create the repositories:
   - `anesis-dev/scoop-anesis` — a Scoop bucket. Scoop reads manifests from
     `bucket/` if it exists, otherwise the repo root. `render.sh` writes to
     `bucket/anesis.json`.
   - `anesis-dev/homebrew-anesis` — a Homebrew tap. The repo name **must** start
     with `homebrew-`; `brew tap anesis-dev/anesis` then resolves to it.
     `render.sh` writes to `Formula/anesis.rb`.
2. Create a fine-grained PAT with `contents: write` on **those two repositories
   only** (not on `anesis-cli`), and add it to `anesis-cli` as the repository
   secret `PACKAGING_TOKEN`.
3. Tag a release. The `packaging` job renders both manifests and pushes them.

Users then install with:

```sh
brew install anesis-dev/anesis/anesis     # macOS / Linux
scoop bucket add anesis https://github.com/anesis-dev/scoop-anesis && scoop install anesis
```

## Testing the substitution without a release

```sh
packaging/render.sh 1.0.0 path/to/SHA256SUMS out/
```

writes the rendered manifests into `out/` for inspection. The release workflow
runs the same script.

## Deliberately not here

- **winget** — every release needs a pull request into `microsoft/winget-pkgs`
  and a human review on Microsoft's side, so it cannot be a fire-and-forget CI
  step. Add it once the release cadence is known.
- **AUR / Nix** — both want a maintainer who actually uses the platform. The
  release already publishes checksummed tarballs, which is all either needs.
