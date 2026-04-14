import * as vscode from "vscode";
import { TopicsTreeProvider } from "./topicsTreeProvider";
import { SummaryTreeProvider } from "./summaryTreeProvider";
import { DueTreeProvider } from "./dueTreeProvider";
import { GraphPanel } from "./graphPanel";
import { QuizPanel } from "./quizPanel";
import { RockyDataProvider } from "./rockyDataProvider";

export function activate(context: vscode.ExtensionContext): void {
  const dataProvider = new RockyDataProvider();

  const topicsTree = new TopicsTreeProvider(dataProvider);
  const summaryTree = new SummaryTreeProvider(dataProvider);
  const dueTree = new DueTreeProvider(dataProvider);

  // ── status bar item ──────────────────────────────────────────────────────
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  statusBar.command = "rocky.refreshTopics";
  statusBar.tooltip = "Rocky: click to refresh knowledge graph";
  updateStatusBar(statusBar, dataProvider);

  // ── auto-refresh on pkg.json changes ────────────────────────────────────
  dataProvider.onDidChange(() => {
    topicsTree.refresh();
    summaryTree.refresh();
    dueTree.refresh();
    updateStatusBar(statusBar, dataProvider);
  });

  // ── hover provider — show topic info when cursor is on a matching word ──
  const hoverProvider = vscode.languages.registerHoverProvider(
    { scheme: "file" },
    {
      provideHover(document, position) {
        const range = document.getWordRangeAtPosition(position, /[A-Za-z][A-Za-z0-9_\- ]{2,}/);
        if (!range) return;
        const word = document.getText(range).trim();
        if (word.length < 3) return;
        const node = dataProvider.getNodeByName(word);
        if (!node) return;
        const pct = (node.retrievability * 100).toFixed(1);
        const icon = node.retrievability > 0.7 ? "🟢" : node.retrievability > 0.4 ? "🟡" : "🔴";
        const md = new vscode.MarkdownString(
          [
            `**Rocky** — ${icon} **${node.name}**`,
            ``,
            `| | |`,
            `|---|---|`,
            `| Domain | ${node.classification ?? "—"} |`,
            `| Retrievability | ${pct}% |`,
            `| Reviews | ${node.total_reviews} |`,
            `| Stability | ${node.stability.toFixed(2)} |`,
            node.canonical_question ? `| Question | ${escapeMarkdown(node.canonical_question)} |` : "",
          ].filter((l) => l !== "").join("\n")
        );
        md.isTrusted = true;
        return new vscode.Hover(md, range);
      },
    }
  );

  context.subscriptions.push(
    dataProvider,
    statusBar,
    hoverProvider,

    vscode.window.registerTreeDataProvider("rockyTopics", topicsTree),
    vscode.window.registerTreeDataProvider("rockySummary", summaryTree),
    vscode.window.registerTreeDataProvider("rockyDue", dueTree),

    // ── refresh ─────────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.refreshTopics", () => {
      dataProvider.clearCache();
      topicsTree.refresh();
      summaryTree.refresh();
      dueTree.refresh();
      updateStatusBar(statusBar, dataProvider);
    }),

    // ── graph ────────────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.openGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "global");
    }),
    vscode.commands.registerCommand("rocky.openGlobalGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "global");
    }),
    vscode.commands.registerCommand("rocky.openLocalGraph", () => {
      GraphPanel.createOrShow(context.extensionUri, dataProvider, "local");
    }),

    // ── quiz ─────────────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.quizWeak", () => {
      QuizPanel.createOrShow(context.extensionUri, dataProvider, "weak");
    }),
    vscode.commands.registerCommand("rocky.quizAll", () => {
      QuizPanel.createOrShow(context.extensionUri, dataProvider, "all");
    }),
    vscode.commands.registerCommand("rocky.quizRepo", async () => {
      const pkg = dataProvider.getPkgData();
      if (!pkg) return;
      const repos = [...new Set(pkg.nodes.map((n) => n.repo).filter(Boolean))] as string[];
      if (repos.length === 0) {
        vscode.window.showWarningMessage("No repos found in knowledge graph.");
        return;
      }
      const picked = await vscode.window.showQuickPick(repos.sort(), {
        placeHolder: "Select a repo to quiz",
        title: "Rocky: Quiz by Repo",
      });
      if (picked) {
        QuizPanel.createOrShow(context.extensionUri, dataProvider, picked);
      }
    }),

    // ── search ───────────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.searchTopics", async () => {
      const pkg = dataProvider.getPkgData();
      if (!pkg) {
        vscode.window.showWarningMessage('Rocky: No data. Run "rocky backup" first.');
        return;
      }
      const items = pkg.nodes
        .sort((a, b) => a.retrievability - b.retrievability)
        .map((n) => {
          const pct = (n.retrievability * 100).toFixed(0);
          const icon = n.retrievability > 0.7 ? "🟢" : n.retrievability > 0.4 ? "🟡" : "🔴";
          return {
            label: `${icon} ${n.name}`,
            description: `${n.classification ?? "Other"} · ${pct}% · ${n.repo ?? "global"}`,
            detail: n.canonical_question ?? undefined,
            name: n.name,
          };
        });

      const picked = await vscode.window.showQuickPick(items, {
        matchOnDescription: true,
        matchOnDetail: true,
        placeHolder: "Search topics by name, domain, or repo…",
        title: "Rocky: Search Topics",
      });
      if (picked) {
        const node = dataProvider.getNodeByName(picked.name);
        if (node) {
          const panel = vscode.window.createWebviewPanel(
            "rockyTopicDetail",
            `Rocky: ${node.name}`,
            vscode.ViewColumn.One,
            {}
          );
          panel.webview.html = buildTopicDetailHtml(node);
        }
      }
    }),

    // ── sort / group toggles ─────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.sortByName", () => {
      topicsTree.setSortMode("name");
      vscode.window.showInformationMessage("Rocky: Topics sorted by name");
    }),
    vscode.commands.registerCommand("rocky.sortByRetrievability", () => {
      topicsTree.setSortMode("retrievability");
      vscode.window.showInformationMessage("Rocky: Topics sorted by retrievability (weakest first)");
    }),
    vscode.commands.registerCommand("rocky.groupByDomain", () => {
      topicsTree.setGroupMode("domain");
    }),
    vscode.commands.registerCommand("rocky.groupByRepo", () => {
      topicsTree.setGroupMode("repo");
    }),

    // ── topic detail ──────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.showTopicDetail", (topic: { name: string }) => {
      const node = dataProvider.getNodeByName(topic.name);
      if (!node) {
        vscode.window.showWarningMessage(`Rocky: Topic "${topic.name}" not found.`);
        return;
      }
      const panel = vscode.window.createWebviewPanel(
        "rockyTopicDetail",
        `Rocky: ${node.name}`,
        vscode.ViewColumn.One,
        {}
      );
      panel.webview.html = buildTopicDetailHtml(node);
    }),

    // ── open web app ──────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.openWebApp", () => {
      const serverUrl = dataProvider.getServerUrl();
      if (serverUrl) {
        vscode.env.openExternal(vscode.Uri.parse(serverUrl));
      } else {
        vscode.window.showInformationMessage(
          "Rocky: Set rocky.serverUrl to open the web app. Start rocky with `rocky view`."
        );
      }
    }),
  );

  statusBar.show();
}

function updateStatusBar(bar: vscode.StatusBarItem, dp: RockyDataProvider): void {
  const pkg = dp.getPkgData();
  if (!pkg) {
    bar.text = "$(database) Rocky";
    return;
  }
  const due = pkg.nodes.filter((n) => n.retrievability < 0.4).length;
  const total = pkg.nodes.length;
  if (due > 0) {
    bar.text = `$(circle-filled) Rocky: ${due} due`;
    bar.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
  } else {
    bar.text = `$(check) Rocky: ${total} topics`;
    bar.backgroundColor = undefined;
  }
}

function buildTopicDetailHtml(node: RockyNode): string {
  const retrievabilityPct = (node.retrievability * 100).toFixed(1);
  const barColor =
    node.retrievability > 0.7 ? "#4caf50" : node.retrievability > 0.4 ? "#ff9800" : "#f44336";
  const reviewsHtml = (node.reviews ?? [])
    .slice(-5)
    .reverse()
    .map(
      (r) => `<tr>
        <td>${escapeHtml(r.date)}</td>
        <td style="color:${r.score >= 0.7 ? "#4caf50" : r.score >= 0.4 ? "#ff9800" : "#f44336"}">${scoreLabel(r.score)}</td>
        ${r.feedback ? `<td style="font-size:11px;color:var(--vscode-descriptionForeground)">${escapeHtml(r.feedback.slice(0, 80))}…</td>` : "<td></td>"}
      </tr>`
    )
    .join("");

  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <style>
    body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); padding: 16px; max-width: 600px; }
    h1 { font-size: 1.3em; margin-bottom: 4px; }
    .badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.85em;
             background: var(--vscode-badge-background); color: var(--vscode-badge-foreground); margin-right: 6px; }
    .bar-container { width: 100%; background: var(--vscode-input-background); border-radius: 4px; margin: 6px 0; }
    .bar { height: 10px; border-radius: 4px; }
    table { border-collapse: collapse; margin-top: 10px; width: 100%; }
    td { padding: 4px 10px 4px 0; vertical-align: top; font-size: 13px; }
    td:first-child { font-weight: 600; white-space: nowrap; }
    th { text-align: left; font-size: 11px; color: var(--vscode-descriptionForeground); padding: 3px 10px 3px 0; }
    .section-h { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: .5px;
                 color: var(--vscode-descriptionForeground); margin: 14px 0 4px; }
    .q-box { padding: 8px; background: var(--vscode-input-background); border-radius: 4px; font-size: 13px; line-height: 1.5; }
  </style>
</head>
<body>
  <h1>${escapeHtml(node.name)}</h1>
  <span class="badge">${escapeHtml(node.classification ?? "Other")}</span>
  <span class="badge">${escapeHtml(node.repo ?? "global")}</span>

  <p class="section-h">Retrievability</p>
  <div class="bar-container">
    <div class="bar" style="width:${retrievabilityPct}%; background:${barColor};"></div>
  </div>
  <p style="font-size:13px">${retrievabilityPct}%</p>

  <table>
    <tr><td>Difficulty</td><td>${node.difficulty.toFixed(2)}</td></tr>
    <tr><td>Stability</td><td>${node.stability.toFixed(2)}</td></tr>
    <tr><td>Total Reviews</td><td>${node.total_reviews}</td></tr>
  </table>

  ${
    node.canonical_question
      ? `<p class="section-h">Canonical Question</p>
         <div class="q-box">${escapeHtml(node.canonical_question)}</div>`
      : ""
  }
  ${
    node.canonical_answer
      ? `<p class="section-h">Reference Answer</p>
         <div class="q-box" style="color:var(--vscode-descriptionForeground)">${escapeHtml(node.canonical_answer)}</div>`
      : ""
  }

  ${
    reviewsHtml
      ? `<p class="section-h">Recent Reviews</p>
         <table>
           <tr><th>Date</th><th>Score</th><th>Feedback</th></tr>
           ${reviewsHtml}
         </table>`
      : ""
  }
</body>
</html>`;
}

function scoreLabel(s: number): string {
  if (s >= 0.9) return "★ Cold";
  if (s >= 0.7) return "✓ Known";
  if (s >= 0.4) return "~ Partly";
  return "✗ Gap";
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function escapeMarkdown(text: string): string {
  return text.replace(/[|*_`[\]]/g, "\\$&");
}

export interface RockyReview {
  date: string;
  score: number;
  feedback?: string;
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
  reviews?: RockyReview[];
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
