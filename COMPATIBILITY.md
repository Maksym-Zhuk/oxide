# Compatibility policy

Anesis has two independent version fields, checked against two different things:

| Field | Lives in | Declares compatibility with |
|---|---|---|
| `anesisVersion` (e.g. `>=0.9.0`) | template manifests (`anesis.template.json`) | the CLI's own version (semver range) |
| `schema_version` (e.g. `"1"`) | addon and stack manifests (`anesis.addon.json`, `anesis.stack.json`) | the *shape* of the manifest format itself, not the CLI version |

## Current behavior

**Both fields are enforced.** The checks live in `src/compat.rs`:

| Field | Checked where | On mismatch |
|---|---|---|
| `schema_version` (addon) | `addons::install` — the single funnel every cached addon passes through | hard error naming the addon, the required schema version and an `anesis upgrade` hint |
| `schema_version` (stack) | `stacks::manifest::validate`, reached by every `load_stack` (registry, cache, and `stack link`) | same |
| `anesisVersion` (template) | `templates::cache::update_templates_cache` at install/link time, and again in `create_new_project` for templates cached by an older CLI | hard error naming the template, the required range and the running version |

Two deliberate softenings:

- A **prerelease** build compares as its release version, so `anesis 1.1.0-rc1`
  satisfies `>=1.0.0`. Plain semver would reject it — exactly the build most
  likely to be testing a new manifest.
- An `anesisVersion` that is **not a valid semver range** produces a warning, not
  an error. It means "we cannot tell", and refusing to scaffold over a typo in
  advisory metadata is worse than proceeding. An unparseable `schema_version`
  *is* an error, because it gates the manifest shape itself.

`SUPPORTED_SCHEMA_VERSION` in `src/compat.rs` is the single source of truth for
what this CLI understands. Bump it in the same change that teaches the CLI a new
manifest shape.

## The frozen v1.0.0 formats

As of this release the three manifest shapes (validated by
`apps/server/schemas/anesis.{template,addon,stack}.schema.json` and by
`registry-lint`) are frozen:

- **Addons / stacks** are at `schema_version: "1"`. For stacks, the frozen `"1"`
  shape requires `version` (semver) and a structured `author` (`name` + `github`)
  alongside `id`, `name`, and `template`. Any future change to the shape of an
  addon or stack manifest (new required field, changed step type, etc.) must bump
  this to `"2"`, not silently change what `"1"` means.
- **Templates** don't carry their own schema version; a template's `anesisVersion`
  instead declares the minimum CLI version its manifest shape requires. The current
  registry uses `>=0.9.0` (CLI's `templates` schema hasn't changed since then).

## Publishing a new manifest shape

When the format changes:

1. Update the JSON Schemas in `apps/server/schemas/` and the vendored copies in
   `tools/registry-lint/schemas/` (CI checks these do not drift).
2. Teach the CLI the new shape and bump `SUPPORTED_SCHEMA_VERSION`.
3. Only then publish manifests carrying the new `schema_version`.

Doing (3) before (2) is now a clean, actionable error for users on an older CLI
rather than a silent partial apply — which is the whole point of enforcing it.
