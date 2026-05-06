import * as vscode from "vscode";
import { TopicsTreeProvider, type ScopeMode } from "./topicsTreeProvider";
import { SummaryTreeProvider } from "./summaryTreeProvider";
import { DueTreeProvider } from "./dueTreeProvider";
import { SessionsTreeProvider } from "./sessionsTreeProvider";
import { QuizPanel } from "./quizPanel";
import { RockyDataProvider } from "./rockyDataProvider";

const TOPIC_SCOPE_KEY = "rocky.topicScope";

export function activate(context: vscode.ExtensionContext): void {
  const dataProvider = new RockyDataProvider();

  const topicsTree = new TopicsTreeProvider(dataProvider);
  const summaryTree = new SummaryTreeProvider(dataProvider);
  const dueTree = new DueTreeProvider(dataProvider);
  const sessionsTree = new SessionsTreeProvider(dataProvider);

  // Restore the previous topic-scope choice (default: repo-only).
  const savedScope = context.workspaceState.get<ScopeMode>(TOPIC_SCOPE_KEY, "repo");
  topicsTree.setScope(savedScope);
  void vscode.commands.executeCommand("setContext", TOPIC_SCOPE_KEY, savedScope);

  // ── status bar item ──────────────────────────────────────────────────────
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
  statusBar.command = "rocky.refreshTopics";
  statusBar.tooltip = "Rocky: click to refresh knowledge graph";
  void updateStatusBar(statusBar, dataProvider);

  // ── auto-refresh on pkg.json changes ────────────────────────────────────
  dataProvider.onDidChange(() => {
    topicsTree.refresh();
    summaryTree.refresh();
    dueTree.refresh();
    sessionsTree.refresh();
    void updateStatusBar(statusBar, dataProvider);
  });

  // Refresh status bar / sessions periodically when a server is configured
  // (so Rocky IQ stays current without forcing a manual refresh).
  const poll = setInterval(() => {
    if (dataProvider.getServerUrl()) {
      void updateStatusBar(statusBar, dataProvider);
      sessionsTree.refresh();
    }
  }, 30_000);
  context.subscriptions.push({ dispose: () => clearInterval(poll) });

  // ── hover provider — show topic info when cursor is on a matching word ──
  const hoverProvider = vscode.languages.registerHoverProvider(
    { scheme: "file" },
    {
      provideHover(document, position) {
        const range = document.getWordRangeAtPosition(position, /[A-Za-z][A-Za-z0-9_\- ]{2,}/);
        if (!range) return;
        const word = document.getText(range).trim();
        if (word.length < 3) return;
        const node = dataProvider.getNodeByTopic(word);
        if (!node) return;
        const pct = (node.retrievability * 100).toFixed(1);
        const icon = node.retrievability > 0.7 ? "🟢" : node.retrievability > 0.4 ? "🟡" : "🔴";
        const md = new vscode.MarkdownString(
          [
            `**Rocky** — ${icon} **${node.topic}**`,
            ``,
            `| | |`,
            `|---|---|`,
            `| Domain | ${node.domain || "—"} |`,
            `| Retrievability | ${pct}% |`,
            `| Reviews | ${node.review_count} |`,
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
    vscode.window.registerTreeDataProvider("rockySessions", sessionsTree),

    // ── refresh ─────────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.refreshTopics", () => {
      dataProvider.clearCache();
      topicsTree.refresh();
      summaryTree.refresh();
      dueTree.refresh();
      sessionsTree.refresh();
      void updateStatusBar(statusBar, dataProvider);
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
        vscode.window.showWarningMessage('Rocky: No data. Run "rocky export" first.');
        return;
      }
      const items = pkg.nodes
        .sort((a, b) => a.retrievability - b.retrievability)
        .map((n) => {
          const pct = (n.retrievability * 100).toFixed(0);
          const icon = n.retrievability > 0.7 ? "🟢" : n.retrievability > 0.4 ? "🟡" : "🔴";
          return {
            label: `${icon} ${n.topic}`,
            description: `${n.domain || "Other"} · ${pct}% · ${n.repo ?? "global"}`,
            detail: n.canonical_question,
            topic: n.topic,
          };
        });

      const picked = await vscode.window.showQuickPick(items, {
        matchOnDescription: true,
        matchOnDetail: true,
        placeHolder: "Search topics by name, domain, or repo…",
        title: "Rocky: Search Topics",
      });
      if (picked) {
        const node = dataProvider.getNodeByTopic(picked.topic);
        if (node) {
          const panel = vscode.window.createWebviewPanel(
            "rockyTopicDetail",
            `Rocky: ${node.topic}`,
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
      topicsTree.setSortMode("recall");
      vscode.window.showInformationMessage("Rocky: Topics sorted by recall (weakest first)");
    }),
    vscode.commands.registerCommand("rocky.groupByDomain", () => {
      topicsTree.setGroupMode("domain");
    }),
    vscode.commands.registerCommand("rocky.groupByRepo", () => {
      topicsTree.setGroupMode("repo");
    }),

    // ── topic scope toggle (repo-only ↔ all-projects) ────────────────────────
    vscode.commands.registerCommand("rocky.showAllProjects", () => {
      setTopicScope(context, topicsTree, "all");
    }),
    vscode.commands.registerCommand("rocky.showRepoOnly", () => {
      setTopicScope(context, topicsTree, "repo");
    }),

    // ── topic detail ──────────────────────────────────────────────────────────
    vscode.commands.registerCommand("rocky.showTopicDetail", (topic: { topic: string }) => {
      const node = dataProvider.getNodeByTopic(topic.topic);
      if (!node) {
        vscode.window.showWarningMessage(`Rocky: Topic "${topic.topic}" not found.`);
        return;
      }
      const panel = vscode.window.createWebviewPanel(
        "rockyTopicDetail",
        `Rocky: ${node.topic}`,
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

    // ── agent-driven quiz / teach (spawns a terminal with claude/opencode) ───
    vscode.commands.registerCommand("rocky.quizInTerminal", () => {
      spawnAgentTerminal("Rocky Quiz", "/rocky-quiz");
    }),
    vscode.commands.registerCommand("rocky.teachInTerminal", () => {
      spawnAgentTerminal("Rocky Teach", "/rocky-teach");
    }),
    vscode.commands.registerCommand(
      "rocky.quizTopicInTerminal",
      (item?: { topicName?: string }) => {
        const topic = item?.topicName;
        if (!topic) {
          vscode.window.showWarningMessage("Rocky: No topic selected.");
          return;
        }
        spawnAgentTerminal(`Rocky Quiz · ${topic}`, `/rocky-quiz me on "${topic}"`);
      }
    ),
    vscode.commands.registerCommand(
      "rocky.teachTopicInTerminal",
      (item?: { topicName?: string }) => {
        const topic = item?.topicName;
        if (!topic) {
          vscode.window.showWarningMessage("Rocky: No topic selected.");
          return;
        }
        spawnAgentTerminal(`Rocky Teach · ${topic}`, `/rocky-teach me about "${topic}"`);
      }
    ),
  );

  statusBar.show();
}

function setTopicScope(
  context: vscode.ExtensionContext,
  topicsTree: TopicsTreeProvider,
  scope: ScopeMode
): void {
  topicsTree.setScope(scope);
  void context.workspaceState.update(TOPIC_SCOPE_KEY, scope);
  void vscode.commands.executeCommand("setContext", TOPIC_SCOPE_KEY, scope);
  vscode.window.showInformationMessage(
    scope === "repo"
      ? "Rocky: showing topics for this repo only."
      : "Rocky: showing topics from all projects."
  );
}

/**
 * Spawn a VSCode terminal with the configured agent CLI and a starter prompt.
 * Defaults to `claude`; override via the `rocky.agentCommand` setting.
 *
 * Verified CLI shapes:
 *   - claude:   `claude "<prompt>"` — positional starter prompt opens an
 *               interactive session with the prompt pre-sent.
 *   - opencode: bare `opencode` opens the TUI; the default positional is a
 *               *project path*, not a message. `opencode run "<msg>"` is
 *               one-shot (non-interactive), unsuitable for quiz/teach.
 *               We launch the TUI and surface the slash command in a
 *               VSCode notification so the user can type it once the TUI
 *               is ready.
 *   - other:    same fallback as opencode — launch bare, show the prompt
 *               in a notification.
 */
function spawnAgentTerminal(name: string, prompt: string): void {
  const cfg = vscode.workspace.getConfiguration("rocky");
  const agent = cfg.get<string>("agentCommand")?.trim() || "claude";
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const terminal = vscode.window.createTerminal({ name, cwd: folder });

  const isClaudeCli = agent.split(/\s+/)[0] === "claude";
  if (isClaudeCli) {
    const safePrompt = prompt.replace(/"/g, '\\"');
    terminal.sendText(`${agent} "${safePrompt}"`);
  } else {
    terminal.sendText(agent);
    void vscode.window.showInformationMessage(
      `Rocky: type  ${prompt}  into the agent once its prompt is ready.`
    );
  }

  terminal.show();
}

async function updateStatusBar(
  bar: vscode.StatusBarItem,
  dp: RockyDataProvider
): Promise<void> {
  // Prefer the live server — Rocky IQ + due-count are only meaningful with
  // up-to-date retrievability values.
  const dashboard = await dp.fetchDashboard();
  if (dashboard) {
    const dueCount = dashboard.dueForReview.filter(
      (d) => d.retrievability < 0.4
    ).length;
    const iq = dashboard.iq;
    const iqIcon = iq >= 80 ? "$(check)" : iq >= 70 ? "$(circle-outline)" : "$(circle-filled)";
    bar.text = `${iqIcon} Rocky IQ: ${iq}${dueCount > 0 ? ` · ${dueCount} due` : ""}`;
    bar.tooltip =
      `Rocky IQ ${iq}/100 — recall over the last 60 days.\n` +
      `Atrophy score: ${(dashboard.atrophyScore * 100).toFixed(0)}%\n` +
      `Click to refresh.`;
    bar.backgroundColor =
      iq < 60
        ? new vscode.ThemeColor("statusBarItem.errorBackground")
        : iq < 70
          ? new vscode.ThemeColor("statusBarItem.warningBackground")
          : undefined;
    return;
  }

  // Fallback: pkg.json — we synthesize retrievability locally so this still works.
  const pkg = dp.getPkgData();
  if (!pkg) {
    bar.text = "$(database) Rocky";
    bar.tooltip = "Rocky: no data yet — run `rocky export` or set rocky.serverUrl";
    bar.backgroundColor = undefined;
    return;
  }
  const due = pkg.nodes.filter((n) => n.retrievability < 0.4).length;
  if (due > 0) {
    bar.text = `$(circle-filled) Rocky: ${due} due`;
    bar.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
  } else {
    bar.text = `$(check) Rocky: ${pkg.nodes.length} topics`;
    bar.backgroundColor = undefined;
  }
  bar.tooltip = "Rocky: click to refresh knowledge graph";
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
  <h1>${escapeHtml(node.topic)}</h1>
  <span class="badge">${escapeHtml(node.domain || "Other")}</span>
  <span class="badge">${escapeHtml(node.repo ?? "global")}</span>

  <p class="section-h">Retrievability</p>
  <div class="bar-container">
    <div class="bar" style="width:${retrievabilityPct}%; background:${barColor};"></div>
  </div>
  <p style="font-size:13px">${retrievabilityPct}%</p>

  <table>
    <tr><td>Difficulty</td><td>${node.difficulty.toFixed(2)}</td></tr>
    <tr><td>Stability</td><td>${node.stability.toFixed(2)}</td></tr>
    <tr><td>Total Reviews</td><td>${node.review_count}</td></tr>
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
  question: string;
  answer: string;
  feedback?: string;
  score: number;
}

/**
 * Mirrors a node entry in `/api/data` (see src/server.rs `get_data`).
 * Snake_case is preserved to keep the wire shape and the TS type identical.
 */
export interface RockyNode {
  id: string;
  topic: string;
  kind: string;
  domain: string;
  description: string;
  stability: number;
  difficulty: number;
  /** Time-decay-since-last-review (FSRS R). */
  retrievability: number;
  /** Mean of last 3 review scores (default 0.5 if no real reviews). */
  mastery?: number;
  /** retrievability × mastery — what we sort/filter "due" by. */
  recall_now?: number;
  /** Class: "known" (≥0.6), "stale" (≥0.3), "gap" (<0.3) — applied to recall_now. */
  classification: "known" | "stale" | "gap";
  last_reviewed: string;
  review_count: number;
  created_at: string;
  /** Repo name. `undefined` when loaded from pkg.json (export schema omits it). */
  repo?: string;
  /** Distinct repos this topic has been encountered in. */
  repos?: string[];
  canonical_question?: string;
  canonical_answer?: string;
  canonical_clue?: string;
  question_bank?: Array<{ question: string; answer: string; clue?: string; asked_count?: number }>;
  reviews?: RockyReview[];
}

/** Compute recall_now from a node, falling back to retrievability when the
 * server didn't supply it (e.g. older exports). */
export function recallNow(n: RockyNode): number {
  if (typeof n.recall_now === "number") return n.recall_now;
  const m = typeof n.mastery === "number" ? n.mastery : 0.5;
  return n.retrievability * m;
}

export interface RockyEdge {
  source: string;
  target: string;
  /** Edge kind from server (`implies`, `depends_on`, `part_of`, `conflicts_with`). */
  relation: string;
}

export interface PkgData {
  nodes: RockyNode[];
  edges: RockyEdge[];
}

// ── /api/data dashboard summary ─────────────────────────────────────────────

export interface DomainHealthEntry {
  domain: string;
  count: number;
  avg_recall: number;
  known: number;
  fading: number;
  gap: number;
}

export interface DueForReviewEntry {
  id: string;
  topic: string;
  domain: string;
  retrievability: number;
  last_reviewed: string;
}

export interface RecentlyAddedEntry {
  id: string;
  topic: string;
  domain: string;
  created_at: string;
  kind: string;
}

export interface RockyDashboard {
  /** Rocky IQ — round((1 - atrophyScore) * 100). Higher is better. */
  iq: number;
  atrophyScore: number;
  domainHealth: DomainHealthEntry[];
  dueForReview: DueForReviewEntry[];
  recentlyAdded: RecentlyAddedEntry[];
  userName: string;
}

// ── /api/sessions ───────────────────────────────────────────────────────────

export interface SessionTopic {
  id: string;
  topic: string;
  domain: string;
  kind: string;
  encounter_count: number;
  source_commits: string[];
  repo: string;
}

export interface RockySession {
  date: string;
  count: number;
  topics: SessionTopic[];
}

export function deactivate(): void {
  // nothing to clean up
}
