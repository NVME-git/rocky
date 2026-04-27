import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { RockyNode } from "./extension";
import { recallNow } from "./extension";

type SortMode = "recall" | "name";
type GroupMode = "domain" | "repo";

export class TopicsTreeProvider implements vscode.TreeDataProvider<TopicItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<TopicItem | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private sortMode: SortMode = "recall";
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
    this.setSortMode(this.sortMode === "recall" ? "name" : "recall");
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

    const groups = new Map<string, RockyNode[]>();
    for (const node of pkg.nodes) {
      const key =
        this.groupMode === "domain"
          ? node.domain || "Other"
          : node.repo || "global";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(node);
    }

    const sortFn =
      this.sortMode === "recall"
        ? (a: RockyNode, b: RockyNode) => recallNow(a) - recallNow(b)
        : (a: RockyNode, b: RockyNode) => a.topic.localeCompare(b.topic);

    const sortedKeys = [...groups.keys()].sort();
    return sortedKeys.map((groupKey) => {
      const nodes = groups.get(groupKey)!.sort(sortFn);

      const children = nodes.map((n) => {
        const recall = recallNow(n);
        const pct = (recall * 100).toFixed(0);
        const icon = recall > 0.6 ? "🟢" : recall > 0.3 ? "🟡" : "🔴";
        const subtitle =
          this.groupMode === "domain" ? (n.repo ?? "") : (n.domain || "Other");
        const item = new TopicItem(
          `${icon} ${n.topic}  (${pct}%)`,
          vscode.TreeItemCollapsibleState.None
        );
        item.description = subtitle;
        const masteryPct = ((n.mastery ?? 0.5) * 100).toFixed(0);
        const rPct = (n.retrievability * 100).toFixed(0);
        item.tooltip = [
          n.topic,
          `Recall: ${pct}%  (R ${rPct}% · M ${masteryPct}%)`,
          `Domain: ${n.domain || "Other"}`,
          `Repo: ${n.repo ?? "global"}`,
          `Difficulty: ${n.difficulty.toFixed(2)}`,
          `Stability: ${n.stability.toFixed(2)}`,
          `Reviews: ${n.review_count}`,
        ].join("\n");
        item.command = {
          command: "rocky.showTopicDetail",
          title: "Show Topic Detail",
          arguments: [{ topic: n.topic }],
        };
        item.contextValue = "rockyTopic";
        return item;
      });

      const weak = nodes.filter((n) => recallNow(n) < 0.3).length;
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
