"use strict";
/**
 * Rocky Browser Extension — Background service worker
 *
 * Handles:
 * - Context menu integration (right-click → Add to Rocky)
 * - Message passing from popup and content scripts
 * - Communication with the Rocky sync endpoint (future)
 */
// Context menu for right-click "Add to Rocky"
chrome.runtime.onInstalled.addListener(() => {
    chrome.contextMenus.create({
        id: "rocky-add-link",
        title: "Add to Rocky PKG",
        contexts: ["link", "page"],
    });
});
chrome.contextMenus.onClicked.addListener((info, tab) => {
    if (info.menuItemId === "rocky-add-link") {
        const url = info.linkUrl ?? info.pageUrl ?? "";
        const title = tab?.title ?? "";
        // Store for the popup to pick up, or process directly
        chrome.storage.local.set({
            pendingResource: { url, title, timestamp: Date.now() },
        });
    }
});
// Handle messages from popup
chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message.type === "ADD_RESOURCE") {
        handleAddResource(message)
            .then((result) => sendResponse(result))
            .catch((err) => sendResponse({ success: false, error: String(err) }));
        return true; // async response
    }
    if (message.type === "SYNC_TOPICS") {
        handleSyncTopics()
            .then((result) => sendResponse(result))
            .catch((err) => sendResponse({ success: false, error: String(err) }));
        return true;
    }
});
async function handleAddResource(message) {
    // In a future version this will POST to a Rocky sync API.
    // For now, store the resource locally for later sync.
    const data = await chrome.storage.local.get("pendingResources");
    const pending = data.pendingResources ?? [];
    pending.push({
        url: message.url,
        title: message.title,
        topics: message.topics,
        addedAt: new Date().toISOString(),
    });
    await chrome.storage.local.set({ pendingResources: pending });
    return { success: true };
}
async function handleSyncTopics() {
    // Placeholder: in the future, fetch topics from a Rocky sync endpoint
    // For now, return current stored topics
    const data = await chrome.storage.local.get("rockyTopics");
    return { success: true, ...(data.rockyTopics ? {} : { error: "No topics synced yet" }) };
}
