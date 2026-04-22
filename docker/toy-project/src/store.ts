import Database from "better-sqlite3";

export interface Store {
  insert(url: string): number;
  lookup(id: number): string | null;
  close(): void;
}

export function openStore(path: string): Store {
  const db = new Database(path);
  db.pragma("journal_mode = WAL");
  db.exec(`
    CREATE TABLE IF NOT EXISTS links (
      id  INTEGER PRIMARY KEY AUTOINCREMENT,
      url TEXT NOT NULL,
      created_at INTEGER NOT NULL DEFAULT (unixepoch())
    );
  `);

  const insertStmt = db.prepare("INSERT INTO links (url) VALUES (?)");
  const lookupStmt = db.prepare("SELECT url FROM links WHERE id = ?");

  return {
    insert(url) {
      const info = insertStmt.run(url);
      return Number(info.lastInsertRowid);
    },
    lookup(id) {
      const row = lookupStmt.get(id) as { url: string } | undefined;
      return row?.url ?? null;
    },
    close() {
      db.close();
    },
  };
}
