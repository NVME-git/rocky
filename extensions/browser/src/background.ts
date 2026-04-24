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
//
// Captures are stored *locally only*. Rocky's server has no resource ingest
// endpoint — the PKG is built from your code, not your browsing. The history
// view is the user's record of pages they tagged.
async function handleAddResource(message: {
  url: string;
  title: string;
  topics: string[];
  note?: string;
}): Promise<{ success: boolean; error?: string }> {
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
  await chrome.storage.local.set({ captureHistory: history.slice(0, 50) });
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
    const json = await resp.json() as {
      nodes?: Array<{ topic: string; kind?: string; retrievability?: number; classification?: string; repo?: string; canonical_question?: string }>;
      atrophyScore?: number;
    };

    const nodes = (json.nodes ?? []).filter(
      (n: { kind?: string }) => n.kind !== "domain" && n.kind !== "user"
    );

    const atrophy = typeof json.atrophyScore === "number" ? json.atrophyScore : 0;
    const iq = Math.round((1 - atrophy) * 100);

    await chrome.storage.local.set({
      rockyNodes: nodes,
      rockyTopics: nodes.map((n) => n.topic),
      rockyIq: iq,
      rockyAtrophy: atrophy,
      lastSynced: new Date().toISOString(),
    });

    // Badge: show IQ instead of due-count — it's the more useful at-a-glance signal.
    updateBadge(iq);

    return { success: true, count: nodes.length };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

/**
 * Badge shows Rocky IQ (0–100). Colour-coded:
 *   ≥80 green, ≥70 amber, <70 red. Empty when nothing has been synced.
 */
function updateBadge(iq: number | null): void {
  if (iq === null || Number.isNaN(iq)) {
    chrome.action.setBadgeText({ text: "" });
    return;
  }
  chrome.action.setBadgeText({ text: String(iq) });
  const colour = iq >= 80 ? "#2ecc71" : iq >= 70 ? "#f39c12" : "#e74c3c";
  chrome.action.setBadgeBackgroundColor({ color: colour });
}

// Restore badge on service-worker startup.
chrome.storage.local.get("rockyIq").then((data) => {
  const iq = data["rockyIq"];
  if (typeof iq === "number") {
    updateBadge(iq);
  }
});
