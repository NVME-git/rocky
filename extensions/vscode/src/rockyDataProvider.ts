import * as path from "path";
import * as fs from "fs";
import * as vscode from "vscode";
import type { PkgData, RockyNode } from "./extension";

/**
 * Provides read access to Rocky's PKG data.
 *
 * Reads the pkg.json backup file that Rocky produces with `rocky backup`.
 * Falls back to running `rocky ls --json` via the CLI if the file is not found.
 */
export class RockyDataProvider {
  private cache: PkgData | null = null;

  /** Return the global ~/.rocky directory, respecting user config. */
  getGlobalRockyDir(): string {
    const configured = vscode.workspace
      .getConfiguration("rocky")
      .get<string>("globalDbPath");

    if (configured) {
      // If the user pointed at graph.db directly, return the parent dir
      if (configured.endsWith("graph.db")) {
        return path.dirname(configured);
      }
      return configured;
    }

    const home =
      process.env.HOME ?? process.env.USERPROFILE ?? "";
    return path.join(home, ".rocky");
  }

  /** Return the local .rocky directory inside the current workspace, if any. */
  getLocalRockyDir(): string | null {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) {
      return null;
    }
    const localDir = path.join(folders[0].uri.fsPath, ".rocky");
    return fs.existsSync(localDir) ? localDir : null;
  }

  /** Load and cache PKG data from the backup JSON. */
  getPkgData(): PkgData | null {
    if (this.cache) {
      return this.cache;
    }

    // Try global pkg.json first
    const globalDir = this.getGlobalRockyDir();
    const pkgPath = path.join(globalDir, "pkg", "pkg.json");

    if (fs.existsSync(pkgPath)) {
      try {
        const raw = fs.readFileSync(pkgPath, "utf-8");
        this.cache = JSON.parse(raw) as PkgData;
        return this.cache;
      } catch (err) {
        vscode.window.showWarningMessage(
          `Rocky: Failed to parse ${pkgPath}: ${err}`
        );
      }
    }

    vscode.window.showInformationMessage(
      'Rocky: No pkg.json found. Run "rocky backup" to generate it.'
    );
    return null;
  }

  /** Get nodes filtered by the current workspace repo name. */
  getLocalNodes(): RockyNode[] {
    const pkg = this.getPkgData();
    if (!pkg) {
      return [];
    }

    const folders = vscode.workspace.workspaceFolders;
    if (!folders || folders.length === 0) {
      return [];
    }

    const repoName = path.basename(folders[0].uri.fsPath);
    return pkg.nodes.filter(
      (n) => n.repo !== null && n.repo.toLowerCase() === repoName.toLowerCase()
    );
  }

  /** Look up a single node by name. */
  getNodeByName(name: string): RockyNode | undefined {
    const pkg = this.getPkgData();
    return pkg?.nodes.find((n) => n.name === name);
  }

  /** Clear the in-memory cache so the next read reloads from disk. */
  clearCache(): void {
    this.cache = null;
  }
}
