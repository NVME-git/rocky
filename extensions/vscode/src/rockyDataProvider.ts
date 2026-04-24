import * as path from "path";
import * as fs from "fs";
import * as vscode from "vscode";
import type {
  PkgData,
  RockyDashboard,
  RockyEdge,
  RockyNode,
  RockySession,
} from "./extension";

/**
 * Provides access to Rocky's PKG data.
 *
 * Two sources, in order of preference:
 *   1. The running `rocky view` server (when `rocky.serverUrl` is set).
 *      Uses the `/api/data` and `/api/sessions` shapes directly.
 *   2. The pkg.json export file (`rocky export`). Older shape — we synthesize
 *      retrievability and FSRS classification on the fly so the rest of the
 *      extension can treat both sources uniformly.
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

  /**
   * Load and cache PKG data from the on-disk pkg.json export.
   * Synthesizes retrievability + classification using the FSRS formula
   * because the export schema doesn't store them.
   */
  getPkgData(): PkgData | null {
    if (this.cache) return this.cache;

    const pkgPath = this.getPkgPath();
    if (!fs.existsSync(pkgPath)) {
      vscode.window.showInformationMessage(
        'Rocky: No pkg.json found. Run "rocky export" to generate it.'
      );
      return null;
    }

    try {
      const raw = fs.readFileSync(pkgPath, "utf-8");
      const parsed = JSON.parse(raw) as PkgExportFile;
      this.cache = {
        nodes: (parsed.nodes ?? []).map(normalizeExportedNode),
        edges: [], // pkg.json export doesn't include edges
      };
      return this.cache;
    } catch (err) {
      vscode.window.showWarningMessage(`Rocky: Failed to parse ${pkgPath}: ${err}`);
      return null;
    }
  }

  /** Fetch live data from the running rocky server, returning null on failure. */
  async fetchLiveData(): Promise<PkgData | null> {
    const serverUrl = this.getServerUrl();
    if (!serverUrl) return null;
    try {
      const resp = await fetch(`${serverUrl}/api/data`);
      if (!resp.ok) return null;
      const data = (await resp.json()) as ServerDataResponse;
      return {
        nodes: (data.nodes ?? [])
          .filter((n) => n.kind !== "domain" && n.kind !== "user")
          .map(normalizeServerNode),
        edges: (data.edges ?? []).map((e) => ({
          source: e.source,
          target: e.target,
          relation: e.kind ?? "",
        })),
      };
    } catch {
      return null;
    }
  }

  /**
   * Fetch the dashboard summary (Rocky IQ, atrophy, due/recent lists)
   * from `/api/data`. Returns null if no server is configured or it's down.
   */
  async fetchDashboard(): Promise<RockyDashboard | null> {
    const serverUrl = this.getServerUrl();
    if (!serverUrl) return null;
    try {
      const resp = await fetch(`${serverUrl}/api/data`);
      if (!resp.ok) return null;
      const data = (await resp.json()) as ServerDataResponse;
      const atrophy = typeof data.atrophyScore === "number" ? data.atrophyScore : 0;
      return {
        iq: Math.round((1 - atrophy) * 100),
        atrophyScore: atrophy,
        domainHealth: data.domainHealth ?? [],
        dueForReview: data.dueForReview ?? [],
        recentlyAdded: data.recentlyAdded ?? [],
        userName: data.userName ?? "",
      };
    } catch {
      return null;
    }
  }

  /** Fetch the per-day session log from `/api/sessions`. */
  async fetchSessions(): Promise<RockySession[]> {
    const serverUrl = this.getServerUrl();
    if (!serverUrl) return [];
    try {
      const resp = await fetch(`${serverUrl}/api/sessions`);
      if (!resp.ok) return [];
      const data = (await resp.json()) as { sessions?: RockySession[] };
      return data.sessions ?? [];
    } catch {
      return [];
    }
  }

  /** Get nodes filtered by the current workspace repo name. */
  getLocalNodes(): RockyNode[] {
    const pkg = this.getPkgData();
    if (!pkg) return [];
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) return [];
    const repoName = path.basename(folders[0].uri.fsPath).toLowerCase();
    return pkg.nodes.filter(
      (n) => n.repo !== undefined && n.repo.toLowerCase() === repoName
    );
  }

  /** Look up a single node by topic name (case-insensitive). */
  getNodeByTopic(topic: string): RockyNode | undefined {
    const pkg = this.getPkgData();
    return pkg?.nodes.find((n) => n.topic.toLowerCase() === topic.toLowerCase());
  }

  /** Look up nodes whose topic name contains the query. */
  searchNodes(query: string): RockyNode[] {
    const pkg = this.getPkgData();
    if (!pkg) return [];
    const q = query.toLowerCase();
    return pkg.nodes.filter((n) => n.topic.toLowerCase().includes(q));
  }

  /** Clear the in-memory cache so the next read reloads from disk. */
  clearCache(): void {
    this.cache = null;
    this._onDidChange.fire();
  }

  private startWatcher(): void {
    const pkgPath = this.getPkgPath();
    const dir = path.dirname(pkgPath);
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
      // non-fatal
    }
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

// ── internal: shape adapters ────────────────────────────────────────────────

interface PkgExportFile {
  version: number;
  exported_at: string;
  nodes: ExportedNode[];
}

interface ExportedNode {
  id: string;
  topic: string;
  kind: string;
  domain: string;
  description: string;
  difficulty: number;
  stability: number;
  last_reviewed: string;
  last_encountered: string;
  review_count: number;
  contexts: string[];
  created_at: string;
}

interface ServerDataResponse {
  userName?: string;
  nodes?: ServerNode[];
  edges?: ServerEdge[];
  atrophyScore?: number;
  domainHealth?: RockyDashboard["domainHealth"];
  dueForReview?: RockyDashboard["dueForReview"];
  recentlyAdded?: RockyDashboard["recentlyAdded"];
}

interface ServerNode extends RockyNode {
  // server already matches RockyNode, listed here for clarity
}

interface ServerEdge {
  source: string;
  target: string;
  kind?: string;
}

function normalizeServerNode(n: ServerNode): RockyNode {
  // /api/data may emit "Other" for empty repo — keep that, but coerce to undefined
  // when there's truly nothing useful.
  return {
    ...n,
    repo: n.repo && n.repo !== "" ? n.repo : undefined,
  };
}

function normalizeExportedNode(n: ExportedNode): RockyNode {
  const r = computeRetrievability(n.stability, n.last_reviewed);
  return {
    id: n.id,
    topic: n.topic,
    kind: n.kind,
    domain: n.domain || "Other",
    description: n.description,
    stability: n.stability,
    difficulty: n.difficulty,
    retrievability: r,
    classification: classify(r),
    last_reviewed: n.last_reviewed,
    review_count: n.review_count,
    created_at: n.created_at,
    repo: undefined, // pkg.json export doesn't carry repo
  };
}

/**
 * FSRS retrievability: R = (1 + t / (9 * S))^-1, where t is days since last
 * review and S is stability. Mirrors `crate::fsrs::retrievability` in Rust.
 */
function computeRetrievability(stability: number, lastReviewed: string): number {
  if (!lastReviewed || stability <= 0) return 1;
  const last = new Date(lastReviewed);
  if (Number.isNaN(last.getTime())) return 1;
  const days = Math.max(0, (Date.now() - last.getTime()) / (1000 * 60 * 60 * 24));
  return 1 / (1 + days / (9 * stability));
}

function classify(r: number): RockyNode["classification"] {
  if (r >= 0.7) return "known";
  if (r >= 0.4) return "stale";
  return "gap";
}
