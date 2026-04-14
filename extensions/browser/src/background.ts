/**
 * Rocky Browser Extension — Background service worker
 *
 * Handles:
 * - Context menus: "Add to Rocky" on page, link, and selected text
 * - Message routing from popup and content scripts
 * - Live topic sync from Rocky server
 * - Badge showing count of topics due for review
 */

// ── context menus ─────────────────────────────────────────────────────────────
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "rocky-add-page",
    title: "Add page to Rocky PKG",
    contexts: ["page"],
  });
  chrome.contextMenus.create({
    id: "rocky-add-link",
    title: "Add link to Rocky PKG",
    contexts: ["link"],
  });
  chrome.contextMenus.create({
    id: "rocky-add-selection",
    title: "Capture selection to Rocky PKG",
    contexts: ["selection"],
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  const url = info.linkUrl ?? info.pageUrl ?? tab?.url ?? "";
  const title = tab?.title ?? "";
  let note = "";

  if (info.menuItemId === "rocky-add-selection") {
    note = info.selectionText ?? "";
  }

  chrome.storage.local.set({
    pendingCapture: { url, title, note, timestamp: Date.now() },
  });

  // Open popup (opens action popup so user can confirm and select topics)
  // We can't programmatically open the popup in MV3, but we set pendingCapture
  // so the next popup open picks it up.
});

// ── message handler ───────────────────────────────────────────────────────────
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === "ADD_RESOURCE") {
    handleAddResource(message)
      .then((r) => sendResponse(r))
      .catch((e) => sendResponse({ success: false, error: String(e) }));
    return true;
  }

  if (message.type === "SYNC_TOPICS_FROM_SERVER") {
    syncTopicsFromServer()
      .then((r) => sendResponse(r))
      .catch((e) => sendResponse({ success: false, error: String(e) }));
    return true;
  }

  if (message.type === "PAGE_DETECTED") {
    // Store last detected page meta for the popup
    chrome.storage.session?.set({ lastPageMeta: message }).catch(() => {
      chrome.storage.local.set({ lastPageMeta: message });
    });
    sendResponse({ ok: true });
    return false;
  }

  if (message.type === "GET_HISTORY") {
    chrome.storage.local.get("captureHistory").then((data) => {
      sendResponse({ history: data["captureHistory"] ?? [] });
    });
    return true;
  }
});

// ── resource storage ──────────────────────────────────────────────────────────
async function handleAddResource(message: {
  url: string;
  title: string;
  topics: string[];
  note?: string;
}): Promise<{ success: boolean; error?: string }> {
  // Try to POST to the rocky server if configured
  const data = await chrome.storage.local.get("rockyServerUrl");
  const serverUrl = (data["rockyServerUrl"] as string | undefined)?.trim();

  // Store locally for history
  const histData = await chrome.storage.local.get("captureHistory");
  const history: Array<{ url: string; title: string; topics: string[]; note?: string; addedAt: string }> =
    (histData["captureHistory"] as typeof history) ?? [];
  history.unshift({
    url: message.url,
    title: message.title,
    topics: message.topics,
    note: message.note,
    addedAt: new Date().toISOString(),
  });
  // Keep last 50 entries
  await chrome.storage.local.set({ captureHistory: history.slice(0, 50) });

  // If server URL is set, POST a resource note to the server
  if (serverUrl) {
    try {
      const resp = await fetch(`${serverUrl}/api/resource`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(message),
      });
      if (resp.ok) return { success: true };
      // Server endpoint may not exist yet — fall through to local-only success
    } catch {
      // Server not reachable — stored locally
    }
  }

  return { success: true };
}

// ── live topic sync from rocky server ────────────────────────────────────────
async function syncTopicsFromServer(): Promise<{ success: boolean; count?: number; error?: string }> {
  const data = await chrome.storage.local.get("rockyServerUrl");
  const serverUrl = (data["rockyServerUrl"] as string | undefined)?.trim();
  if (!serverUrl) {
    return { success: false, error: "No server URL configured" };
  }

  try {
    const resp = await fetch(`${serverUrl}/api/data`);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const json = await resp.json() as { nodes?: Array<{ topic: string; kind?: string; retrievability?: number; classification?: string; repo?: string; canonical_question?: string }> };

    const nodes = (json.nodes ?? []).filter(
      (n: { kind?: string }) => n.kind !== "domain" && n.kind !== "user"
    );

    // Store full node data for richer popup display
    await chrome.storage.local.set({
      rockyNodes: nodes,
      rockyTopics: nodes.map((n) => n.topic),
      lastSynced: new Date().toISOString(),
    });

    // Update badge with due count
    const due = nodes.filter((n) => (n.retrievability ?? 0) < 0.4).length;
    updateBadge(due);

    return { success: true, count: nodes.length };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

function updateBadge(dueCount: number): void {
  if (dueCount > 0) {
    chrome.action.setBadgeText({ text: String(dueCount) });
    chrome.action.setBadgeBackgroundColor({ color: "#e74c3c" });
  } else {
    chrome.action.setBadgeText({ text: "" });
  }
}

// Sync badge on startup
chrome.storage.local.get("rockyNodes").then((data) => {
  const nodes = (data["rockyNodes"] as Array<{ retrievability?: number }> | undefined) ?? [];
  const due = nodes.filter((n) => (n.retrievability ?? 0) < 0.4).length;
  updateBadge(due);
});
