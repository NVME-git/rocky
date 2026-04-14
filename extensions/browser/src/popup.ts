/**
 * Rocky Browser Extension — Popup script
 *
 * Features:
 * - Capture tab: URL/title/note, suggested topics based on page keywords,
 *   full topic search with retrievability dots, multi-select and add
 * - History tab: last 50 captures with topics
 * - Settings tab: configure rocky server URL, sync topics live
 */

interface NodeData {
  topic: string;
  kind?: string;
  retrievability?: number;
  classification?: string;
  repo?: string;
  canonical_question?: string;
}

interface CaptureRecord {
  url: string;
  title: string;
  topics: string[];
  note?: string;
  addedAt: string;
}

interface RockyPageMeta {
  url?: string;
  title?: string;
  description?: string;
  source?: string;
  keywords?: string[];
  selectedText?: string;
}

let allNodes: NodeData[] = [];
let allTopics: string[] = [];
let selectedTopics = new Set<string>();
let currentTab = "capture";

document.addEventListener("DOMContentLoaded", async () => {
  // ── Load stored data ─────────────────────────────────────────────────────
  const stored = await chrome.storage.local.get([
    "rockyNodes", "rockyTopics", "rockyServerUrl", "lastSynced", "captureHistory", "pendingCapture",
  ]);

  allNodes = (stored["rockyNodes"] as NodeData[] | undefined) ?? [];
  allTopics = (stored["rockyTopics"] as string[] | undefined) ?? [];
  const serverUrl = (stored["rockyServerUrl"] as string | undefined) ?? "";
  const lastSynced = (stored["lastSynced"] as string | undefined) ?? "";

  // ── Settings panel ────────────────────────────────────────────────────────
  const serverInput = document.getElementById("server-url-input") as HTMLInputElement;
  serverInput.value = serverUrl;

  const syncInfo = document.getElementById("sync-info")!;
  if (lastSynced) {
    syncInfo.textContent = `Last synced: ${new Date(lastSynced).toLocaleString()} · ${allTopics.length} topics`;
  }
  const topicsInfo = document.getElementById("topics-info")!;
  topicsInfo.textContent = allTopics.length > 0
    ? `${allTopics.length} topics loaded (${allNodes.filter((n) => (n.retrievability ?? 0) < 0.4).length} due for review)`
    : "No topics loaded. Configure server URL and sync.";

  document.getElementById("save-url-btn")!.addEventListener("click", async () => {
    const url = serverInput.value.trim().replace(/\/$/, "");
    await chrome.storage.local.set({ rockyServerUrl: url });
    syncInfo.textContent = "Saved. Click Sync to load topics.";
  });

  document.getElementById("sync-btn")!.addEventListener("click", async () => {
    syncInfo.textContent = "Syncing…";
    const resp = await chrome.runtime.sendMessage({ type: "SYNC_TOPICS_FROM_SERVER" }) as { success: boolean; count?: number; error?: string };
    if (resp.success) {
      syncInfo.textContent = `✓ Synced ${resp.count} topics`;
      // Reload nodes
      const fresh = await chrome.storage.local.get(["rockyNodes", "rockyTopics"]);
      allNodes = (fresh["rockyNodes"] as NodeData[] | undefined) ?? [];
      allTopics = (fresh["rockyTopics"] as string[] | undefined) ?? [];
      topicsInfo.textContent = `${allTopics.length} topics · ${allNodes.filter((n) => (n.retrievability ?? 0) < 0.4).length} due`;
    } else {
      syncInfo.textContent = `✗ ${resp.error}`;
    }
  });

  // ── Server connection indicator ───────────────────────────────────────────
  updateServerDot(serverUrl);

  // ── Capture tab ───────────────────────────────────────────────────────────
  const urlInput = document.getElementById("url") as HTMLInputElement;
  const titleInput = document.getElementById("title") as HTMLInputElement;
  const noteInput = document.getElementById("note") as HTMLTextAreaElement;
  const searchInput = document.getElementById("search") as HTMLInputElement;
  const topicList = document.getElementById("topic-list")!;
  const suggestionsList = document.getElementById("suggestions-list")!;
  const addBtn = document.getElementById("add-btn") as HTMLButtonElement;
  const statusEl = document.getElementById("status")!;
  const sourceBadge = document.getElementById("source-badge")!;

  // Get current tab meta
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

  // Try to get enriched meta from content script
  let pageMeta: RockyPageMeta = { url: tab?.url ?? "", title: tab?.title ?? "" };
  try {
    if (tab?.id) {
      const meta = await chrome.tabs.sendMessage(tab.id, { type: "GET_META" }) as RockyPageMeta | null;
      if (meta) pageMeta = meta;
    }
  } catch {
    // Content script not available on this page
  }

  // Check for pending capture from context menu
  const pending = stored["pendingCapture"] as { url: string; title: string; note: string; timestamp: number } | undefined;
  if (pending && Date.now() - pending.timestamp < 30000) {
    pageMeta.url = pending.url;
    pageMeta.title = pending.title;
    noteInput.value = pending.note ?? "";
    await chrome.storage.local.remove("pendingCapture");
  }

  urlInput.value = pageMeta.url ?? "";
  titleInput.value = pageMeta.title ?? "";
  sourceBadge.textContent = pageMeta.source ?? "web";

  // Fill selection text as note if present
  if (pageMeta.selectedText && !noteInput.value) {
    noteInput.value = pageMeta.selectedText;
  }

  // ── Smart suggestions based on page keywords ──────────────────────────────
  const keywords = pageMeta.keywords ?? [];
  if (keywords.length > 0 && allNodes.length > 0) {
    const suggestions = findSuggestedTopics(keywords, allNodes);
    if (suggestions.length > 0) {
      document.getElementById("suggestions-label")!.style.display = "block";
      suggestionsList.style.display = "flex";
      renderChips(suggestions, suggestionsList, true);
    }
  }

  // ── Topic list rendering ──────────────────────────────────────────────────
  function renderTopics(filter: string): void {
    const filtered = filter
      ? allNodes.filter((n) => n.topic.toLowerCase().includes(filter.toLowerCase()))
      : allNodes.slice(0, 80); // show first 80 when no filter

    document.getElementById("topic-count")!.textContent =
      filter ? `${filtered.length} of ${allNodes.length}` : `${allNodes.length}`;

    topicList.innerHTML = "";
    if (allNodes.length === 0) {
      topicList.innerHTML = '<em style="font-size:11px;color:#6c7086;">No topics — sync with Rocky server in Settings</em>';
      return;
    }
    if (filtered.length === 0) {
      topicList.innerHTML = '<em style="font-size:11px;color:#6c7086;">No matching topics</em>';
      return;
    }
    renderChips(filtered, topicList, false);
  }

  function renderChips(nodes: NodeData[], container: HTMLElement, isSuggested: boolean): void {
    for (const node of nodes) {
      const chip = document.createElement("span");
      const rClass =
        (node.retrievability ?? 0) >= 0.7 ? "r-known"
        : (node.retrievability ?? 0) >= 0.4 ? "r-stale"
        : "r-gap";

      chip.className = "topic-chip" +
        (selectedTopics.has(node.topic) ? " selected" : "") +
        (isSuggested ? " suggested" : "");

      const dot = document.createElement("span");
      dot.className = `r-dot ${node.retrievability !== undefined ? rClass : ""}`;

      chip.appendChild(dot);
      chip.appendChild(document.createTextNode(node.topic));
      chip.title = [
        node.topic,
        node.classification ? `Domain: ${node.classification}` : "",
        node.repo ? `Repo: ${node.repo}` : "",
        node.retrievability !== undefined ? `Retrievability: ${Math.round(node.retrievability * 100)}%` : "",
        node.canonical_question ? `Q: ${node.canonical_question}` : "",
      ].filter(Boolean).join("\n");

      chip.addEventListener("click", () => {
        if (selectedTopics.has(node.topic)) {
          selectedTopics.delete(node.topic);
          chip.classList.remove("selected");
        } else {
          selectedTopics.add(node.topic);
          chip.classList.add("selected");
        }
        // Sync selection state across both lists
        syncChipSelection();
        addBtn.disabled = selectedTopics.size === 0;
      });
      container.appendChild(chip);
    }
  }

  function syncChipSelection(): void {
    document.querySelectorAll<HTMLElement>(".topic-chip").forEach((chip) => {
      const name = chip.textContent?.trim() ?? "";
      chip.classList.toggle("selected", selectedTopics.has(name));
    });
  }

  renderTopics("");
  if (allTopics.length > 0) addBtn.disabled = false;

  searchInput.addEventListener("input", () => renderTopics(searchInput.value));

  // ── Add resource ──────────────────────────────────────────────────────────
  addBtn.addEventListener("click", async () => {
    if (allTopics.length > 0 && selectedTopics.size === 0) {
      statusEl.textContent = "Select at least one topic to link.";
      statusEl.className = "status error";
      return;
    }
    addBtn.disabled = true;
    statusEl.textContent = "Saving…";
    statusEl.className = "status";

    try {
      const resp = await chrome.runtime.sendMessage({
        type: "ADD_RESOURCE",
        url: urlInput.value,
        title: titleInput.value,
        topics: [...selectedTopics],
        note: noteInput.value.trim() || undefined,
      }) as { success: boolean; error?: string };

      if (resp.success) {
        statusEl.textContent = "✓ Saved to knowledge graph!";
        statusEl.className = "status ok";
        selectedTopics.clear();
        syncChipSelection();
      } else {
        statusEl.textContent = resp.error ?? "Failed to save.";
        statusEl.className = "status error";
        addBtn.disabled = false;
      }
    } catch (err) {
      statusEl.textContent = `Error: ${err}`;
      statusEl.className = "status error";
      addBtn.disabled = false;
    }
  });

  // ── History tab ───────────────────────────────────────────────────────────
  const histResp = await chrome.runtime.sendMessage({ type: "GET_HISTORY" }) as { history: CaptureRecord[] };
  renderHistory(histResp.history ?? []);
});

function switchTab(name: string): void {
  currentTab = name;
  document.querySelectorAll<HTMLElement>(".tab").forEach((t, i) => {
    const labels = ["capture", "history", "settings"];
    t.classList.toggle("active", labels[i] === name);
  });
  document.querySelectorAll<HTMLElement>(".panel").forEach((p) => p.classList.remove("active"));
  document.getElementById(`panel-${name}`)?.classList.add("active");
}
(window as Window & typeof globalThis & { switchTab: typeof switchTab }).switchTab = switchTab;

function renderHistory(history: CaptureRecord[]): void {
  const list = document.getElementById("history-list")!;
  if (history.length === 0) {
    list.innerHTML = '<div class="empty-state">No captures yet</div>';
    return;
  }
  list.innerHTML = history.slice(0, 20).map((h) => {
    const date = new Date(h.addedAt).toLocaleDateString();
    const topicsHtml = h.topics.slice(0, 6).map((t) =>
      `<span class="history-topic">${escapeHtml(t)}</span>`
    ).join("");
    return `
      <div class="history-item">
        <div class="history-title" title="${escapeHtml(h.url)}">${escapeHtml(h.title || h.url)}</div>
        <div class="history-meta">
          <span>${date}</span>
          ${h.topics.length > 0 ? `<span>${h.topics.length} topics</span>` : ""}
        </div>
        ${h.topics.length > 0 ? `<div class="history-topics">${topicsHtml}</div>` : ""}
        ${h.note ? `<div style="font-size:10px;color:#6c7086;margin-top:3px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">"${escapeHtml(h.note.slice(0, 80))}"</div>` : ""}
      </div>`;
  }).join("");
}

function updateServerDot(serverUrl: string): void {
  const dot = document.getElementById("server-dot")!;
  const label = document.getElementById("server-label")!;
  if (!serverUrl) {
    dot.className = "server-dot";
    label.textContent = "not connected";
    return;
  }
  // Fire and forget ping
  fetch(`${serverUrl}/api/data`, { signal: AbortSignal.timeout(2000) })
    .then((r) => {
      if (r.ok) {
        dot.className = "server-dot connected";
        label.textContent = "connected";
      } else {
        dot.className = "server-dot error";
        label.textContent = "unreachable";
      }
    })
    .catch(() => {
      dot.className = "server-dot error";
      label.textContent = "unreachable";
    });
}

/** Match page keywords against known topic names to generate suggestions. */
function findSuggestedTopics(keywords: string[], nodes: NodeData[]): NodeData[] {
  const kws = keywords.map((k) => k.toLowerCase());
  const scored: Array<{ node: NodeData; score: number }> = [];

  for (const node of nodes) {
    const name = node.topic.toLowerCase();
    let score = 0;
    for (const kw of kws) {
      if (name === kw) score += 3;
      else if (name.includes(kw) || kw.includes(name)) score += 1;
    }
    if (score > 0) scored.push({ node, score });
  }

  return scored
    .sort((a, b) => b.score - a.score || (b.node.retrievability ?? 0) - (a.node.retrievability ?? 0))
    .slice(0, 8)
    .map((s) => s.node);
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
