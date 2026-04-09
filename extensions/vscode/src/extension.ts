import * as vscode from "vscode";
import { TopicsTreeProvider } from "./topicsTreeProvider";
import { SummaryTreeProvider } from "./summaryTreeProvider";
import { GraphPanel } from "./graphPanel";
import { RockyDataProvider } from "./rockyDataProvider";

export function activate(context: vscode.ExtensionContext): void {
  const dataProvider = new RockyDataProvider();

  const topicsTree = new TopicsTreeProvider(dataProvider);
  const summaryTree = new SummaryTreeProvider(dataProvider);

  context.subscriptions.push(
    vscode.window.registerTreeDataProvider("rockyTopics", topicsTree),
    vscode.window.registerTreeDataProvider("rockySummary", summaryTree),

    vscode.commands.registerCommand("rocky.refreshTopics", () => {
      dataProvider.clearCache();
      topicsTree.refresh();
      summaryTree.refresh();
    }),

    vscode.commands.registerCommand("rocky.openGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "global");
    }),

    vscode.commands.registerCommand("rocky.openGlobalGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "global");
    }),

    vscode.commands.registerCommand("rocky.openLocalGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "local");
    }),

    vscode.commands.registerCommand(
      "rocky.showTopicDetail",
      (topic: { name: string }) => {
        const node = dataProvider.getNodeByName(topic.name);
        if (!node) {
          vscode.window.showWarningMessage(
            `Topic "${topic.name}" not found in the knowledge graph.`
          );
          return;
        }

        const panel = vscode.window.createWebviewPanel(
          "rockyTopicDetail",
          `Rocky: ${node.name}`,
          vscode.ViewColumn.One,
          {}
        );
        panel.webview.html = buildTopicDetailHtml(node);
      }
    )
  );

  vscode.window.showInformationMessage("Rocky extension activated 🪨");
}

function buildTopicDetailHtml(node: RockyNode): string {
  const retrievabilityPct = (node.retrievability * 100).toFixed(1);
  const barColor =
    node.retrievability > 0.7
      ? "#4caf50"
      : node.retrievability > 0.4
        ? "#ff9800"
        : "#f44336";

  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <style>
    body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); padding: 16px; }
    h1 { font-size: 1.4em; margin-bottom: 4px; }
    .badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.85em;
             background: var(--vscode-badge-background); color: var(--vscode-badge-foreground); margin-right: 6px; }
    .bar-container { width: 100%; background: var(--vscode-input-background); border-radius: 4px; margin: 8px 0; }
    .bar { height: 12px; border-radius: 4px; }
    table { border-collapse: collapse; margin-top: 12px; }
    td { padding: 4px 12px 4px 0; vertical-align: top; }
    td:first-child { font-weight: bold; white-space: nowrap; }
  </style>
</head>
<body>
  <h1>${escapeHtml(node.name)}</h1>
  <span class="badge">${escapeHtml(node.classification)}</span>
  <span class="badge">${escapeHtml(node.repo ?? "global")}</span>

  <h2>Retrievability</h2>
  <div class="bar-container">
    <div class="bar" style="width:${retrievabilityPct}%; background:${barColor};"></div>
  </div>
  <p>${retrievabilityPct}%</p>

  <table>
    <tr><td>Difficulty</td><td>${node.difficulty.toFixed(2)}</td></tr>
    <tr><td>Stability</td><td>${node.stability.toFixed(2)}</td></tr>
    <tr><td>Total Reviews</td><td>${node.total_reviews}</td></tr>
    ${node.canonical_question ? `<tr><td>Question</td><td>${escapeHtml(node.canonical_question)}</td></tr>` : ""}
    ${node.canonical_answer ? `<tr><td>Answer</td><td>${escapeHtml(node.canonical_answer)}</td></tr>` : ""}
  </table>
</body>
</html>`;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export interface RockyNode {
  name: string;
  classification: string;
  difficulty: number;
  stability: number;
  retrievability: number;
  total_reviews: number;
  repo: string | null;
  canonical_question: string | null;
  canonical_answer: string | null;
}

export interface RockyEdge {
  source: string;
  target: string;
  relation: string;
}

export interface PkgData {
  nodes: RockyNode[];
  edges: RockyEdge[];
}

export function deactivate(): void {
  // nothing to clean up
}
