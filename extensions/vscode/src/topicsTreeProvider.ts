import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { RockyNode } from "./extension";

export class TopicsTreeProvider
  implements vscode.TreeDataProvider<TopicItem>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    TopicItem | undefined | void
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: TopicItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: TopicItem): Promise<TopicItem[]> {
    if (element) {
      // Children of a domain group — individual topics
      return element.children ?? [];
    }

    const pkg = this.dataProvider.getPkgData();
    if (!pkg || pkg.nodes.length === 0) {
      return [
        new TopicItem(
          "No topics found. Run rocky to build your knowledge graph.",
          vscode.TreeItemCollapsibleState.None
        ),
      ];
    }

    // Group nodes by classification (domain)
    const groups = new Map<string, RockyNode[]>();
    for (const node of pkg.nodes) {
      const key = node.classification || "Other";
      if (!groups.has(key)) {
        groups.set(key, []);
      }
      groups.get(key)!.push(node);
    }

    // Sort groups alphabetically, sort nodes within by retrievability (ascending = weakest first)
    const sortedKeys = [...groups.keys()].sort();
    return sortedKeys.map((domain) => {
      const nodes = groups.get(domain)!;
      nodes.sort((a, b) => a.retrievability - b.retrievability);

      const children = nodes.map((n) => {
        const pct = (n.retrievability * 100).toFixed(0);
        const icon = n.retrievability > 0.7 ? "🟢" : n.retrievability > 0.4 ? "🟡" : "🔴";
        const item = new TopicItem(
          `${icon} ${n.name}  (${pct}%)`,
          vscode.TreeItemCollapsibleState.None
        );
        item.tooltip = `${n.name}\nRetrievability: ${pct}%\nDifficulty: ${n.difficulty.toFixed(2)}\nStability: ${n.stability.toFixed(2)}\nReviews: ${n.total_reviews}`;
        item.command = {
          command: "rocky.showTopicDetail",
          title: "Show Topic Detail",
          arguments: [{ name: n.name }],
        };
        return item;
      });

      const groupItem = new TopicItem(
        `${domain} (${nodes.length})`,
        vscode.TreeItemCollapsibleState.Collapsed
      );
      groupItem.children = children;
      return groupItem;
    });
  }
}

export class TopicItem extends vscode.TreeItem {
  children?: TopicItem[];

  constructor(
    label: string,
    collapsibleState: vscode.TreeItemCollapsibleState
  ) {
    super(label, collapsibleState);
  }
}
