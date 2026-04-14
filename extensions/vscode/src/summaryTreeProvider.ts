import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";

export class SummaryTreeProvider implements vscode.TreeDataProvider<SummaryItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<SummaryItem | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: SummaryItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: SummaryItem): Promise<SummaryItem[]> {
    if (element?.children) {
      return element.children;
    }

    const pkg = this.dataProvider.getPkgData();
    if (!pkg || pkg.nodes.length === 0) {
      return [new SummaryItem("No data available", "", [])];
    }

    const nodes = pkg.nodes;
    const totalTopics = nodes.length;
    const totalEdges = pkg.edges.length;
    const avgR = nodes.reduce((s, n) => s + n.retrievability, 0) / totalTopics;
    const known = nodes.filter((n) => n.retrievability >= 0.7).length;
    const stale = nodes.filter((n) => n.retrievability >= 0.4 && n.retrievability < 0.7).length;
    const gap = nodes.filter((n) => n.retrievability < 0.4).length;
    const totalReviews = nodes.reduce((s, n) => s + n.total_reviews, 0);
    const neverReviewed = nodes.filter((n) => n.total_reviews === 0).length;

    const domains = [...new Set(nodes.map((n) => n.classification || "Other"))].sort();
    const repos = [...new Set(nodes.map((n) => n.repo).filter(Boolean))] as string[];

    // Per-repo breakdown
    const repoChildren: SummaryItem[] = repos.sort().map((repo) => {
      const repoNodes = nodes.filter((n) => n.repo === repo);
      const rKnown = repoNodes.filter((n) => n.retrievability >= 0.7).length;
      const rGap = repoNodes.filter((n) => n.retrievability < 0.4).length;
      const rAvg = repoNodes.reduce((s, n) => s + n.retrievability, 0) / repoNodes.length;
      return new SummaryItem(
        `  ${repo}`,
        `${repoNodes.length} topics · ${Math.round(rAvg * 100)}% avg · ${rGap} due`,
        []
      );
    });

    const serverUrl = this.dataProvider.getServerUrl();

    return [
      new SummaryItem("📊 Total Topics", `${totalTopics}`, []),
      new SummaryItem("🔗 Connections", `${totalEdges}`, []),
      new SummaryItem("🏷️ Domains", `${domains.length}`, []),
      new SummaryItem("📦 Repos", `${repos.length}`, repoChildren),
      new SummaryItem("📈 Avg Retrievability", `${(avgR * 100).toFixed(1)}%`, []),
      new SummaryItem("🟢 Known (≥70%)", `${known}`, []),
      new SummaryItem("🟡 Fading (40–70%)", `${stale}`, []),
      new SummaryItem("🔴 Gap (<40%)", `${gap}`, []),
      new SummaryItem("📝 Total Reviews", `${totalReviews}`, []),
      new SummaryItem("🆕 Never Reviewed", `${neverReviewed}`, []),
      new SummaryItem(
        "🌐 Server",
        serverUrl ? serverUrl : "Not configured (rocky.serverUrl)",
        []
      ),
    ];
  }
}

class SummaryItem extends vscode.TreeItem {
  children: SummaryItem[];

  constructor(label: string, value: string, children: SummaryItem[]) {
    super(
      `${label}: ${value}`,
      children.length > 0
        ? vscode.TreeItemCollapsibleState.Collapsed
        : vscode.TreeItemCollapsibleState.None
    );
    this.children = children;
    this.description = "";
  }
}
