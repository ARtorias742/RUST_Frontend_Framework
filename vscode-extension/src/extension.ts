import * as vscode from "vscode";

const DOCS_URL = "https://ferrum-rs.github.io/ferrum/docs/";
const DEMO_URL = "https://ferrum-rs.github.io/ferrum/demo/";

function cliPath(): string {
  return vscode.workspace.getConfiguration("ferrum").get<string>("cliPath", "ferrum");
}

function runInTerminal(command: string, name: string) {
  const terminal = vscode.window.createTerminal(name);
  terminal.sendText(command);
  terminal.show(true);
}

function showPanel(context: vscode.ExtensionContext) {
  const panel = vscode.window.createWebviewPanel(
    "ferrumPanel",
    "Ferrum",
    vscode.ViewColumn.Active,
    { enableScripts: true },
  );

  panel.webview.html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Ferrum</title>
<style>
  body { font-family: system-ui, sans-serif; padding: 20px; line-height: 1.55; }
  h1 { margin: 0 0 4px; font-size: 20px; }
  .badge { display: inline-block; padding: 1px 8px; border-radius: 999px; background: #f59e0b22; color: #f59e0b; font-size: 12px; }
  code { background: #ffffff11; padding: 2px 6px; border-radius: 4px; }
  a { color: #60a5fa; }
  ul { padding-left: 18px; }
  .grid { display: grid; gap: 10px; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); margin-top: 16px; }
  .card { border: 1px solid #ffffff22; border-radius: 8px; padding: 12px; }
  .muted { opacity: 0.7; font-size: 12px; }
</style>
</head>
<body>
  <h1>Ferrum <span class="badge">0.1 alpha</span></h1>
  <p class="muted">A Rust frontend framework. Not production-ready.</p>

  <p>Install the CLI:</p>
  <p><code>cargo install --git https://github.com/ferrum-rs/ferrum --bin ferrum --locked</code></p>

  <div class="grid">
    <div class="card"><strong>Dev server</strong><br /><span class="muted">Command palette → Ferrum: Start dev server</span></div>
    <div class="card"><strong>Build</strong><br /><span class="muted">Ferrum: Build for wasm (release)</span></div>
    <div class="card"><strong>Docs</strong><br /><a href="${DOCS_URL}">Open documentation</a></div>
    <div class="card"><strong>Demo</strong><br /><a href="${DEMO_URL}">Open live demo</a></div>
  </div>

  <ul>
    <li>30 components, hooks, router, data, SSR, DevTools</li>
    <li>20/20 React-subset conformance checks</li>
    <li>192 KB gzip demo bundle (alpha budget 200 KB)</li>
  </ul>
</body>
</html>`;
}

export function activate(context: vscode.ExtensionContext) {
  const commands: [string, () => void][] = [
    [
      "ferrum.dev",
      () => {
        const port = vscode.workspace.getConfiguration("ferrum").get<number>("devPort", 8080);
        runInTerminal(`${cliPath()} dev --port ${port}`, "Ferrum dev");
      },
    ],
    ["ferrum.build", () => runInTerminal(`${cliPath()} build`, "Ferrum build")],
    ["ferrum.check", () => runInTerminal(`${cliPath()} check`, "Ferrum check")],
    [
      "ferrum.new",
      async () => {
        const name = await vscode.window.showInputBox({
          prompt: "Ferrum app name",
          placeHolder: "my-app",
        });
        if (name) runInTerminal(`${cliPath()} new ${name}`, "Ferrum new");
      },
    ],
    [
      "ferrum.openDocs",
      () => vscode.env.openExternal(vscode.Uri.parse(DOCS_URL)),
    ],
    [
      "ferrum.openDemo",
      () => vscode.env.openExternal(vscode.Uri.parse(DEMO_URL)),
    ],
    ["ferrum.showPanel", () => showPanel(context)],
  ];

  for (const [id, handler] of commands) {
    context.subscriptions.push(vscode.commands.registerCommand(id, handler));
  }
}

export function deactivate() {}