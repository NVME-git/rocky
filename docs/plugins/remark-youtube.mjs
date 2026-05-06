// remark plugin: rewrite ```youtube VIDEO_ID``` fenced code blocks into
// a 16:9 iframe (privacy-enhanced youtube-nocookie.com). When the ID is
// empty / "PLACEHOLDER" / "TODO" / "TODO_*" / "PLACEHOLDER_*", emits a
// "coming soon" placeholder card instead of an iframe.
import { visit } from 'unist-util-visit';

function escapeAttr(s) {
  return String(s).replace(/[&<>"]/g, c => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;',
  }[c]));
}

function isPlaceholder(id) {
  const v = id.trim().toUpperCase();
  return v === '' || v === 'PLACEHOLDER' || v === 'TODO'
      || v.startsWith('PLACEHOLDER_') || v.startsWith('TODO_');
}

function placeholderHtml() {
  return `<div class="yt-wrap"><div class="yt-placeholder">
<svg viewBox="0 0 24 24" width="64" height="64" aria-hidden="true"><path fill="currentColor" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>
<p class="ph-title">Demo video coming soon</p>
<p class="ph-sub">A walkthrough is in production</p>
</div></div>`;
}

function iframeHtml(id) {
  const safe = escapeAttr(id);
  return `<div class="yt-wrap"><iframe
src="https://www.youtube-nocookie.com/embed/${safe}"
title="YouTube video"
allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
allowfullscreen
loading="lazy"
referrerpolicy="strict-origin-when-cross-origin"></iframe></div>`;
}

export function remarkYouTube() {
  return (tree) => {
    visit(tree, 'code', (node, index, parent) => {
      if (!parent || typeof index !== 'number') return;
      if (node.lang !== 'youtube') return;
      const id = (node.value || '').trim();
      const html = isPlaceholder(id) ? placeholderHtml() : iframeHtml(id);
      parent.children.splice(index, 1, { type: 'html', value: html });
    });
  };
}
