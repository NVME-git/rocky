import * as path from "path";
import * as fs from "fs";
import * as vscode from "vscode";
import type { PkgData, RockyNode } from "./extension";

/**
 * Provides read access to Rocky's PKG data.
 *
 * Reads the pkg.json backup file that Rocky produces with `rocky backup`.
 * Watches the file for changes and fires `onDidChange` when it updates.
 * Also exposes the configured rocky server URL for live API calls.
 */
export class RockyDataProvider {
  private cache: PkgData | null = null;
  private watcher: fs.FSWatcher | null = null;

  private _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChange = this._onDidChange.event;

  constructor() {
    this.startWatcher();
  }

  /** Return the global ~/.rocky directory, respecting user config. */
  getGlobalRockyDir(): string {
    const configured = vscode.workspace
      .getConfiguration("rocky")
      .get<string>("globalDbPath");

    if (configured) {
      if (configured.endsWith("graph.db")) {
        return path.dirname(configured);
      }
      return configured;
    }

    const home = process.env.HOME ?? process.env.USERPROFILE ?? "";
    return path.join(home, ".rocky");
  }

  /** Return the local .rocky directory inside the current workspace, if any. */
  getLocalRockyDir(): string | null {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) return null;
    const localDir = path.join(folders[0].uri.fsPath, ".rocky");
    return fs.existsSync(localDir) ? localDir : null;
  }

  /** Return the pkg.json path. */
  getPkgPath(): string {
    return path.join(this.getGlobalRockyDir(), "pkg", "pkg.json");
  }

  /** Return the configured rocky server URL, or null. */
  getServerUrl(): string | null {
    const url = vscode.workspace
      .getConfiguration("rocky")
      .get<string>("serverUrl");
    return url?.trim() || null;
  }

  /** Load and cache PKG data from the backup JSON. */
  getPkgData(): PkgData | null {
    if (this.cache) return this.cache;

    const pkgPath = this.getPkgPath();
    if (fs.existsSync(pkgPath)) {
      try {
        const raw = fs.readFileSync(pkgPath, "utf-8");
        this.cache = JSON.parse(raw) as PkgData;
        return this.cache;
      } catch (err) {
        vscode.window.showWarningMessage(`Rocky: Failed to parse ${pkgPath}: ${err}`);
      }
    } else {
      vscode.window.showInformationMessage(
        'Rocky: No pkg.json found. Run "rocky backup" to generate it.'
      );
    }
    return null;
  }

  /** Fetch live data from the running rocky server, returning null on failure. */
  async fetchLiveData(): Promise<PkgData | null> {
    const serverUrl = this.getServerUrl();
    if (!serverUrl) return null;
    try {
      const resp = await fetch(`${serverUrl}/api/data`);
      if (!resp.ok) return null;
      const data = await resp.json() as { nodes?: RockyNode[]; edges?: { source: string; target: string; relation: string }[] };
      // Server returns {nodes, edges, ...} — normalize to PkgData shape
      return {
        nodes: (data.nodes ?? []).filter(
          (n: RockyNode & { kind?: string }) => n.kind !== "domain" && n.kind !== "user"
        ) as RockyNode[],
        edges: (data.edges ?? []).map((e: { source: string; target: string; kind?: string; relation?: string }) => ({
          source: e.source,
          target: e.target,
          relation: e.kind ?? e.relation ?? "",
        })),
      };
    } catch {
      return null;
    }
  }

  /** Get nodes filtered by the current workspace repo name. */
  getLocalNodes(): RockyNode[] {
    const pkg = this.getPkgData();
    if (!pkg) return [];
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) return [];
    const repoName = path.basename(folders[0].uri.fsPath);
    return pkg.nodes.filter(
      (n) => n.repo !== null && n.repo.toLowerCase() === repoName.toLowerCase()
    );
  }

  /** Look up a single node by name (case-insensitive). */
  getNodeByName(name: string): RockyNode | undefined {
    const pkg = this.getPkgData();
    return pkg?.nodes.find(
      (n) => n.name.toLowerCase() === name.toLowerCase()
    );
  }

  /** Look up nodes whose name contains the query. */
  searchNodes(query: string): RockyNode[] {
    const pkg = this.getPkgData();
    if (!pkg) return [];
    const q = query.toLowerCase();
    return pkg.nodes.filter((n) => n.name.toLowerCase().includes(q));
  }

  /** Clear the in-memory cache so the next read reloads from disk. */
  clearCache(): void {
    this.cache = null;
    this._onDidChange.fire();
  }

  /** Start watching pkg.json for external changes (e.g. after `rocky backup`). */
  private startWatcher(): void {
    const pkgPath = this.getPkgPath();
    const dir = path.dirname(pkgPath);

    // Watch the directory so we catch creates too
    try {
      if (fs.existsSync(dir)) {
        this.watcher = fs.watch(dir, (_event, filename) => {
          if (filename === "pkg.json" || filename === null) {
            this.cache = null;
            this._onDidChange.fire();
          }
        });
      }
    } catch {
      // If watching fails (permissions, etc.) it's non-fatal
    }

    // Also watch the workspace .rocky directory if present
    const localDir = this.getLocalRockyDir();
    if (localDir) {
      try {
        fs.watch(localDir, () => {
          this.cache = null;
          this._onDidChange.fire();
        });
      } catch {
        // non-fatal
      }
    }
  }

  dispose(): void {
    this.watcher?.close();
    this._onDidChange.dispose();
  }
}
