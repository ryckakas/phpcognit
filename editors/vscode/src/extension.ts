import * as vscode from "vscode";
import { resolveBinary, type Binary } from "./binary";
import { scan, type Finding } from "./run";

const MISSING_BINARY_DISMISSED = "phpcognit.missingBinaryDismissed";

let diagnostics: vscode.DiagnosticCollection;
let binary: Binary | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  diagnostics = vscode.languages.createDiagnosticCollection("phpcognit");
  context.subscriptions.push(diagnostics);

  binary = await resolveBinary(settings().path);
  if (binary === undefined) {
    await reportMissingBinary(context);
  }

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => void refresh(document)),
    vscode.workspace.onDidSaveTextDocument((document) => void refresh(document)),
    vscode.workspace.onDidCloseTextDocument((document) => diagnostics.delete(document.uri)),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("phpcognit")) {
        binary = await resolveBinary(settings().path);
        await refreshAll();
      }
    }),
  );

  await refreshAll();
}

export function deactivate(): void {
  diagnostics?.dispose();
}

function settings(): { enable: boolean; path: string; threshold: number } {
  const config = vscode.workspace.getConfiguration("phpcognit");
  return {
    enable: config.get<boolean>("enable", true),
    path: config.get<string>("path", ""),
    threshold: config.get<number>("threshold", 15),
  };
}

async function refreshAll(): Promise<void> {
  await Promise.all(vscode.workspace.textDocuments.map((document) => refresh(document)));
}

async function refresh(document: vscode.TextDocument): Promise<void> {
  const { enable, threshold } = settings();

  if (!enable || document.languageId !== "php" || document.uri.scheme !== "file") {
    diagnostics.delete(document.uri);
    return;
  }

  const workspace = vscode.workspace.getWorkspaceFolder(document.uri);
  if (workspace === undefined || binary === undefined) {
    return;
  }

  try {
    const findings = await scan({
      binary,
      file: document.uri.fsPath,
      workspaceRoot: workspace.uri.fsPath,
      threshold,
    });

    diagnostics.set(document.uri, findings.map((finding) => toDiagnostic(document, finding)));
  } catch (error) {
    // A broken scan should not leave stale diagnostics implying the file is clean.
    diagnostics.delete(document.uri);
    console.error("phpcognit:", error);
  }
}

function toDiagnostic(document: vscode.TextDocument, finding: Finding): vscode.Diagnostic {
  const line = Math.max(0, Math.min(finding.line - 1, document.lineCount - 1));
  const range = document.lineAt(line).range;

  const diagnostic = new vscode.Diagnostic(
    range,
    `${finding.name} has a cognitive complexity of ${finding.score}`,
    vscode.DiagnosticSeverity.Warning,
  );
  diagnostic.source = "phpcognit";
  diagnostic.code = "cognitive-complexity";

  return diagnostic;
}

/**
 * Shown once per installation. Staying silent makes the extension look broken;
 * warning on every activation nags anyone who has not installed the tool yet.
 */
async function reportMissingBinary(context: vscode.ExtensionContext): Promise<void> {
  if (context.globalState.get<boolean>(MISSING_BINARY_DISMISSED) === true) {
    return;
  }

  const install = "Installation instructions";
  const dismiss = "Don't show again";

  const choice = await vscode.window.showWarningMessage(
    "phpcognit was not found. Install it, or set `phpcognit.path` to an existing binary.",
    install,
    dismiss,
  );

  if (choice === install) {
    await vscode.env.openExternal(
      vscode.Uri.parse("https://github.com/ryckakas/phpcognit#install"),
    );
  } else if (choice === dismiss) {
    await context.globalState.update(MISSING_BINARY_DISMISSED, true);
  }
}
