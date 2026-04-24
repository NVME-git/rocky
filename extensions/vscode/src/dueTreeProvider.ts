import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { RockyNode } from "./extension";

/**
 * Shows topics that are due for review (retrievability < 0.5), sorted weakest first.
 * Grouped by repo so it's clear which project each topic belongs to.
 */
export class DueTreeProvider implements vscode.TreeDataProvider<DueItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<DueItem | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: DueItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: DueItem): Promise<DueItem[]> {
    if (element?.children) {
      return element.children;
    }

    const pkg = this.dataProvider.getPkgData();
    if (!pkg || pkg.nodes.length === 0) {
      return [new DueItem("No data. Run rocky to build your knowledge graph.", [], "info")];
    }

    const dueNodes = pkg.nodes
      .filter((n) => n.retrievability < 0.5)
      .sort((a, b) => a.retrievability - b.retrievability);

    if (dueNodes.length === 0) {
      return [new DueItem("✅ Nothing due for review — great work!", [], "info")];
    }

    const groups = new Map<string, RockyNode[]>();
    for (const n of dueNodes) {
      const key = n.repo ?? "global";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(n);
    }

    const items: DueItem[] = [];
    for (const [repo, nodes] of [...groups.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
      const children: DueItem[] = nodes.map((n) => {
        const pct = (n.retrievability * 100).toFixed(0);
        const icon = n.retrievability > 0.4 ? "🟡" : "🔴";
        const item = new DueItem(`${icon} ${n.topic}  (${pct}%)`, [], "topic");
        item.tooltip = [
          n.topic,
          `Retrievability: ${pct}%`,
          `Domain: ${n.domain || "Other"}`,
          `Reviews: ${n.review_count}`,
          n.canonical_question ? `Q: ${n.canonical_question}` : "",
        ].filter(Boolean).join("\n");
        item.command = {
          command: "rocky.showTopicDetail",
          title: "Show Topic Detail",
          arguments: [{ topic: n.topic }],
        };
        return item;
      });

      const groupItem = new DueItem(
        `${repo}  (${nodes.length} due)`,
        children,
        "repo"
      );
      items.push(groupItem);
    }
    return items;
  }
}

export class DueItem extends vscode.TreeItem {
  children: DueItem[];

  constructor(label: string, children: DueItem[], kind: "topic" | "repo" | "info") {
    const state =
      kind === "repo"
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None;
    super(label, state);
    this.children = children;
    if (kind === "repo") {
      this.iconPath = new vscode.ThemeIcon("repo");
    } else if (kind === "topic") {
      this.iconPath = new vscode.ThemeIcon("circle-outline");
    } else {
      this.iconPath = new vscode.ThemeIcon("info");
    }
  }
}
