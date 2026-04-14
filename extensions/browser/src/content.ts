/**
 * Rocky Browser Extension — Content script
 *
 * Detects page metadata across multiple site types and sends it to the background.
 * Supports: YouTube, GitHub, MDN, Wikipedia, Hacker News, arXiv, dev.to, Stack Overflow.
 */

interface RockyPageMeta {
  url?: string;
  title?: string;
  description?: string;
  source?: string;
  keywords?: string[];
  selectedText?: string;
}

function extractMeta(): RockyPageMeta | null {
  const url = window.location.href;
  const host = window.location.hostname;

  let title = document.title.trim();
  let description = "";
  let source = "web";
  let keywords: string[] = [];

  // ── YouTube ──────────────────────────────────────────────────────────────
  if (host.includes("youtube.com")) {
    if (!url.includes("/watch")) return null;
    const titleEl = document.querySelector("h1.ytd-watch-metadata yt-formatted-string");
    const channelEl = document.querySelector("ytd-channel-name yt-formatted-string a");
    title = titleEl?.textContent?.trim() ?? document.title;
    description = channelEl?.textContent?.trim() ?? "";
    source = "youtube";
    keywords = extractKeywordsFromTitle(title);
  }

  // ── GitHub ────────────────────────────────────────────────────────────────
  else if (host.includes("github.com")) {
    const repoDesc = document.querySelector('[data-testid="repo-description"] p, .f4.my-3');
    const topics = [...document.querySelectorAll('[data-octo-click="topic_click"] a, .topic-tag')]
      .map((el) => el.textContent?.trim() ?? "")
      .filter(Boolean);
    description = repoDesc?.textContent?.trim() ?? "";
    keywords = topics.length > 0 ? topics : extractKeywordsFromTitle(title);
    source = "github";
    // Clean GitHub title format: "user/repo: description"
    title = title.replace(/ · GitHub$/, "").trim();
  }

  // ── MDN Web Docs ──────────────────────────────────────────────────────────
  else if (host.includes("developer.mozilla.org")) {
    const h1 = document.querySelector("h1");
    title = h1?.textContent?.trim() ?? title;
    const summary = document.querySelector(".section-content p");
    description = summary?.textContent?.trim().slice(0, 200) ?? "";
    source = "mdn";
    keywords = extractKeywordsFromTitle(title);
  }

  // ── Wikipedia ─────────────────────────────────────────────────────────────
  else if (host.includes("wikipedia.org")) {
    const h1 = document.querySelector("#firstHeading");
    title = h1?.textContent?.trim() ?? title;
    const firstPara = document.querySelector("#mw-content-text .mw-parser-output > p:not(.mw-empty-elt)");
    description = firstPara?.textContent?.trim().slice(0, 200) ?? "";
    source = "wikipedia";
    keywords = extractKeywordsFromTitle(title);
  }

  // ── Hacker News ───────────────────────────────────────────────────────────
  else if (host.includes("news.ycombinator.com")) {
    const titleEl = document.querySelector(".titleline > a, .storylink");
    title = titleEl?.textContent?.trim() ?? title;
    source = "hackernews";
    keywords = extractKeywordsFromTitle(title);
  }

  // ── arXiv ─────────────────────────────────────────────────────────────────
  else if (host.includes("arxiv.org")) {
    const h1 = document.querySelector("h1.title.mathjax, .title");
    title = h1?.textContent?.replace(/^Title:\s*/i, "").trim() ?? title;
    const abs = document.querySelector(".abstract.mathjax, blockquote.abstract");
    description = abs?.textContent?.replace(/^Abstract:\s*/i, "").trim().slice(0, 200) ?? "";
    source = "arxiv";
    keywords = extractKeywordsFromTitle(title);
  }

  // ── dev.to ────────────────────────────────────────────────────────────────
  else if (host.includes("dev.to")) {
    const h1 = document.querySelector("h1");
    title = h1?.textContent?.trim() ?? title;
    const tags = [...document.querySelectorAll(".tags a, .tag")]
      .map((el) => el.textContent?.trim() ?? "")
      .filter(Boolean);
    keywords = tags.length > 0 ? tags : extractKeywordsFromTitle(title);
    source = "devto";
  }

  // ── Stack Overflow ────────────────────────────────────────────────────────
  else if (host.includes("stackoverflow.com") || host.includes("stackexchange.com")) {
    const h1 = document.querySelector("h1[itemprop='name'], #question-header h1");
    title = h1?.textContent?.trim() ?? title;
    const tags = [...document.querySelectorAll(".post-tag, .tags a")]
      .map((el) => el.textContent?.trim() ?? "")
      .filter(Boolean);
    keywords = tags;
    source = "stackoverflow";
  }

  // ── generic fallback ──────────────────────────────────────────────────────
  else {
    const metaDesc = document.querySelector('meta[name="description"]');
    description = metaDesc?.getAttribute("content")?.slice(0, 200) ?? "";
    const metaKeywords = document.querySelector('meta[name="keywords"]');
    const mk = metaKeywords?.getAttribute("content") ?? "";
    keywords = mk ? mk.split(",").map((k) => k.trim()).filter(Boolean) : extractKeywordsFromTitle(title);
    source = "web";
  }

  const selectedText = window.getSelection()?.toString().trim().slice(0, 500) ?? "";

  return { url, title, description, source, keywords, selectedText };
}

/** Extract candidate topic keywords from a page title. */
function extractKeywordsFromTitle(title: string): string[] {
  // Remove common stop words and split on common separators
  const stopWords = new Set([
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
    "as", "it", "its", "this", "that", "how", "what", "why", "when", "where",
    "i", "you", "we", "they", "he", "she", "my", "your", "our", "their",
    "–", "-", "|", ":", "vs", "vs.", "via", "using", "about", "into",
  ]);

  return title
    .toLowerCase()
    .replace(/[^a-z0-9 .#+\-]/g, " ")
    .split(/\s+/)
    .map((w) => w.trim())
    .filter((w) => w.length > 2 && !stopWords.has(w))
    .slice(0, 10);
}

let lastUrl = window.location.href;

function notify(): void {
  const meta = extractMeta();
  if (!meta) return;
  chrome.runtime.sendMessage({ type: "PAGE_DETECTED", ...meta });
}

// Watch for SPA navigation
const observer = new MutationObserver(() => {
  if (window.location.href !== lastUrl) {
    lastUrl = window.location.href;
    setTimeout(notify, 1500);
  }
});
observer.observe(document.body, { childList: true, subtree: true });

// Initial detection
if (document.readyState === "complete") {
  notify();
} else {
  window.addEventListener("load", notify);
}

// Expose selection text on demand (for context menu captures)
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === "GET_SELECTION") {
    sendResponse({ text: window.getSelection()?.toString().trim() ?? "" });
  }
  if (msg.type === "GET_META") {
    sendResponse(extractMeta());
  }
});
