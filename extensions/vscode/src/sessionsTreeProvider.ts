import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { RockySession, SessionTopic } from "./extension";

/**
 * Per-day session log backed by the running rocky server's `/api/sessions`.
 * Each day shows the topics that landed in the PKG that day, grouped by repo.
 *
 * Requires the `rocky.serverUrl` setting — sessions data is only available
 * from the live server, not from the pkg.json export.
 */
export class SessionsTreeProvider implements vscode.TreeDataProvider<SessionItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<SessionItem | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: SessionItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: SessionItem): Promise<SessionItem[]> {
    if (element?.children) return element.children;
    if (element) return [];

    if (!this.dataProvider.getServerUrl()) {
      return [
        new SessionItem(
          "Set rocky.serverUrl to see session history",
          [],
          "info"
        ),
      ];
    }

    const sessions = await this.dataProvider.fetchSessions();
    if (sessions.length === 0) {
      return [
        new SessionItem("No sessions yet — make some commits", [], "info"),
      ];
    }

    return sessions.map((s) => buildSessionDay(s));
  }
}

function buildSessionDay(s: RockySession): SessionItem {
  const byRepo = new Map<string, SessionTopic[]>();
  for (const t of s.topics) {
    const r = t.repo || "Other";
    if (!byRepo.has(r)) byRepo.set(r, []);
    byRepo.get(r)!.push(t);
  }

  const repoChildren: SessionItem[] = [];
  for (const [repo, topics] of [...byRepo.entries()].sort((a, b) =>
    a[0].localeCompare(b[0])
  )) {
    const topicItems = topics.map((t) => {
      const item = new SessionItem(`◇ ${t.topic}`, [], "topic");
      item.description = t.domain || "";
      item.tooltip = [
        t.topic,
        `Domain: ${t.domain || "Other"}`,
        `Encounters: ${t.encounter_count}`,
        t.source_commits.length
          ? `Commits: ${t.source_commits.slice(0, 3).join(", ")}${t.source_commits.length > 3 ? "…" : ""}`
          : "",
      ]
        .filter(Boolean)
        .join("\n");
      item.command = {
        command: "rocky.showTopicDetail",
        title: "Show Topic Detail",
        arguments: [{ topic: t.topic }],
      };
      return item;
    });

    const repoItem = new SessionItem(
      `${repo}  (${topics.length})`,
      topicItems,
      "repo"
    );
    repoChildren.push(repoItem);
  }

  const day = new SessionItem(
    `📅 ${s.date}  (${s.count} topics)`,
    repoChildren,
    "day"
  );
  return day;
}

export class SessionItem extends vscode.TreeItem {
  children: SessionItem[];

  constructor(label: string, children: SessionItem[], kind: "day" | "repo" | "topic" | "info") {
    const state =
      kind === "day"
        ? vscode.TreeItemCollapsibleState.Collapsed
        : kind === "repo"
          ? vscode.TreeItemCollapsibleState.Collapsed
          : vscode.TreeItemCollapsibleState.None;
    super(label, state);
    this.children = children;
    if (kind === "day") {
      this.iconPath = new vscode.ThemeIcon("calendar");
    } else if (kind === "repo") {
      this.iconPath = new vscode.ThemeIcon("repo");
    } else if (kind === "topic") {
      this.iconPath = new vscode.ThemeIcon("circle-outline");
    } else {
      this.iconPath = new vscode.ThemeIcon("info");
    }
  }
}
