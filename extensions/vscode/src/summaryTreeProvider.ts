import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";

export class SummaryTreeProvider
  implements vscode.TreeDataProvider<SummaryItem>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    SummaryItem | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: SummaryItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: SummaryItem): Promise<SummaryItem[]> {
    if (element) {
      return [];
    }

    const pkg = this.dataProvider.getPkgData();
    if (!pkg || pkg.nodes.length === 0) {
      return [
        new SummaryItem("No data available", ""),
      ];
    }

    const totalTopics = pkg.nodes.length;
    const totalEdges = pkg.edges.length;
    const avgRetrievability =
      pkg.nodes.reduce((sum, n) => sum + n.retrievability, 0) / totalTopics;
    const weakTopics = pkg.nodes.filter((n) => n.retrievability < 0.4).length;
    const strongTopics = pkg.nodes.filter((n) => n.retrievability > 0.7).length;
    const totalReviews = pkg.nodes.reduce((sum, n) => sum + n.total_reviews, 0);

    const domains = new Set(pkg.nodes.map((n) => n.classification || "Other"));

    return [
      new SummaryItem("📊 Total Topics", `${totalTopics}`),
      new SummaryItem("🔗 Total Connections", `${totalEdges}`),
      new SummaryItem("🏷️ Domains", `${domains.size}`),
      new SummaryItem(
        "📈 Avg Retrievability",
        `${(avgRetrievability * 100).toFixed(1)}%`
      ),
      new SummaryItem("🟢 Strong (>70%)", `${strongTopics}`),
      new SummaryItem("🔴 Weak (<40%)", `${weakTopics}`),
      new SummaryItem("📝 Total Reviews", `${totalReviews}`),
    ];
  }
}

class SummaryItem extends vscode.TreeItem {
  constructor(label: string, value: string) {
    super(`${label}: ${value}`, vscode.TreeItemCollapsibleState.None);
    this.description = "";
  }
}
