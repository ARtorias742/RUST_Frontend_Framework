# Ferrum for VS Code

Commands, snippets and `rsx!` highlighting for the Ferrum Rust frontend
framework (**0.1 alpha**).

## Install (from VSIX)

1. Download `ferrum-0.1.0.vsix` from the
   [alpha release](https://github.com/ferrum-rs/ferrum/releases).
2. In VS Code: **Extensions** panel → **⋯** → **Install from VSIX…**.
3. Reload the window.

Or from the CLI:

```bash
code --install-extension ferrum-0.1.0.vsix
```

## Commands

| Command | What it does |
|---|---|
| `Ferrum: Start dev server (HMR)` | `ferrum dev --port <port>` in a terminal |
| `Ferrum: Build for wasm (release)` | `ferrum build` |
| `Ferrum: Check for wasm` | `ferrum check` |
| `Ferrum: New app from template` | prompts for a name, runs `ferrum new` |
| `Ferrum: Open documentation` | opens the docs site |
| `Ferrum: Open live demo` | opens the task-board demo |
| `Ferrum: Show panel` | an in-editor overview panel |

Settings:

- `ferrum.cliPath` (default `ferrum`)
- `ferrum.devPort` (default `8080`)

## Snippets

`ferrum-component`, `ferrum-rsx`, `ferrum-use-state`, `ferrum-use-effect`.

## Develop the extension

```bash
cd vscode-extension
npm install
npm run build
npm run watch      # rebuild on change
# Press F5 to launch the Extension Development Host
```

Package a VSIX:

```bash
npm run package    # -> ferrum-0.1.0.vsix
```

Type-check:

```bash
npm run typecheck
```

## Publish (maintainer)

```bash
npx vsce publish -p <VSCE_PAT>
```

The extension is Apache-2.0. Alpha APIs may change.