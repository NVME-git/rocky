import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { RockyNode } from "./extension";

type SortMode = "retrievability" | "name";
type GroupMode = "domain" | "repo";

export class TopicsTreeProvider implements vscode.TreeDataProvider<TopicItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<TopicItem | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private sortMode: SortMode = "retrievability";
  private groupMode: GroupMode = "domain";

  constructor(private readonly dataProvider: RockyDataProvider) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  setSortMode(mode: SortMode): void {
    this.sortMode = mode;
    this.refresh();
  }

  setGroupMode(mode: GroupMode): void {
    this.groupMode = mode;
    this.refresh();
  }

  toggleSort(): void {
    this.setSortMode(this.sortMode === "retrievability" ? "name" : "retrievability");
  }

  toggleGroup(): void {
    this.setGroupMode(this.groupMode === "domain" ? "repo" : "domain");
  }

  getTreeItem(element: TopicItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: TopicItem): Promise<TopicItem[]> {
    if (element) {
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

    // Group nodes
    const groups = new Map<string, RockyNode[]>();
    for (const node of pkg.nodes) {
      const key =
        this.groupMode === "domain"
          ? node.classification || "Other"
          : node.repo || "global";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(node);
    }

    // Sort within groups
    const sortFn =
      this.sortMode === "retrievability"
        ? (a: RockyNode, b: RockyNode) => a.retrievability - b.retrievability
        : (a: RockyNode, b: RockyNode) => a.name.localeCompare(b.name);

    const sortedKeys = [...groups.keys()].sort();
    return sortedKeys.map((groupKey) => {
      const nodes = groups.get(groupKey)!.sort(sortFn);

      const children = nodes.map((n) => {
        const pct = (n.retrievability * 100).toFixed(0);
        const icon = n.retrievability > 0.7 ? "🟢" : n.retrievability > 0.4 ? "🟡" : "🔴";
        const subtitle = this.groupMode === "domain" ? (n.repo ?? "") : (n.classification ?? "");
        const item = new TopicItem(
          `${icon} ${n.name}  (${pct}%)`,
          vscode.TreeItemCollapsibleState.None
        );
        item.description = subtitle;
        item.tooltip = [
          n.name,
          `Retrievability: ${pct}%`,
          `Domain: ${n.classification || "Other"}`,
          `Repo: ${n.repo ?? "global"}`,
          `Difficulty: ${n.difficulty.toFixed(2)}`,
          `Stability: ${n.stability.toFixed(2)}`,
          `Reviews: ${n.total_reviews}`,
        ].join("\n");
        item.command = {
          command: "rocky.showTopicDetail",
          title: "Show Topic Detail",
          arguments: [{ name: n.name }],
        };
        item.contextValue = "rockyTopic";
        return item;
      });

      const weak = nodes.filter((n) => n.retrievability < 0.5).length;
      const groupItem = new TopicItem(
        `${groupKey}  (${nodes.length}${weak > 0 ? ` · ${weak} due` : ""})`,
        vscode.TreeItemCollapsibleState.Collapsed
      );
      groupItem.children = children;
      groupItem.iconPath =
        this.groupMode === "domain"
          ? new vscode.ThemeIcon("symbol-class")
          : new vscode.ThemeIcon("repo");
      groupItem.contextValue = "rockyGroup";
      return groupItem;
    });
  }
}

export class TopicItem extends vscode.TreeItem {
  children?: TopicItem[];

  constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState) {
    super(label, collapsibleState);
  }
}
