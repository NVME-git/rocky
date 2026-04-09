"use strict";
/**
 * Rocky Browser Extension — Content script
 *
 * Injected into YouTube pages to detect video metadata and provide
 * quick-add functionality for linking videos to PKG topics.
 *
 * This script extracts the video title and URL, then makes them
 * available to the popup and background service worker.
 */
function extractYouTubeMetadata() {
    const url = window.location.href;
    if (!url.includes("youtube.com/watch")) {
        return null;
    }
    const titleEl = document.querySelector("h1.ytd-watch-metadata yt-formatted-string");
    const channelEl = document.querySelector("ytd-channel-name yt-formatted-string a");
    return {
        url,
        title: titleEl?.textContent?.trim() ?? document.title,
        channel: channelEl?.textContent?.trim() ?? "",
    };
}
// Notify the background script when a YouTube video is loaded
function notifyVideoDetected() {
    const meta = extractYouTubeMetadata();
    if (!meta) {
        return;
    }
    chrome.runtime.sendMessage({
        type: "VIDEO_DETECTED",
        ...meta,
    });
}
// YouTube uses SPA navigation, so we watch for URL changes
let lastUrl = window.location.href;
const observer = new MutationObserver(() => {
    if (window.location.href !== lastUrl) {
        lastUrl = window.location.href;
        // Wait for DOM to update after SPA navigation
        setTimeout(notifyVideoDetected, 2000);
    }
});
observer.observe(document.body, { childList: true, subtree: true });
// Initial detection
if (document.readyState === "complete") {
    notifyVideoDetected();
}
else {
    window.addEventListener("load", notifyVideoDetected);
}
