# Anesis

Anesis is a Rust CLI for scaffolding projects from remote templates and extending them
with reusable, versioned project addons.

## Install

```bash
curl -sSL https://raw.githubusercontent.com/anesis-dev/anesis/main/install.sh | bash   # Linux/macOS
irm https://raw.githubusercontent.com/anesis-dev/anesis/main/install.ps1 | iex          # Windows
npm install -g anesis-cli                                                              # npm
cargo install anesis                                                                   # cargo
```

## Quick start

```bash
anesis login                    # authenticate (required for remote templates/addons)
anesis new my-app react-vite-ts # scaffold a project from a template
cd my-app
anesis addon install drizzle
anesis use drizzle init         # apply an addon command to the project
anesis status                   # show the project's template + applied addons
```

## Commands

```text
anesis new <NAME> [TEMPLATE]        create a project (--stack to scaffold template + addons)
anesis template <install|list|info|remove|publish|update>
anesis addon <install|list|info|test|remove|publish|update>
anesis stack <install|list|info|remove|publish|update>
anesis use [ADDON] [COMMAND]        run an addon command in the current project
anesis undo <ADDON>                 revert an applied addon's changes
anesis outdated / anesis update <ADDON>
anesis search [QUERY]               search templates/addons/stacks
anesis login / logout / account
anesis mcp                          run an MCP stdio server for AI agents
anesis completions <shell>          bash, zsh, fish, powershell
anesis info / anesis status
```

Short aliases: `n` new, `t` template, `a` addon, `s` stack, `in` login, `out` logout.
Most commands accept `--json` for machine-readable output and `--yes` to skip prompts.

Full command reference: [npm/README.md](./npm/README.md). Docs and template/addon
catalog: https://anesis-dev.vercel.app.

## Local data

Anesis stores state under `~/.anesis/`: cached templates/addons, `auth.json`, and a
version-check cache. Inside a scaffolded project, applied addons are tracked in
`anesis.lock`.

## License

[PolyForm Noncommercial License 1.0.0](LICENSE.md) — source available, noncommercial
use only.
