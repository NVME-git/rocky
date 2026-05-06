// remark plugin: rewrite `:::details TITLE\n...content...\n:::` blocks
// into native <details><summary>TITLE</summary>...content...</details>
// sequences. Handles two source layouts:
//
//   (a) standalone marker — opener and closer each on their own paragraph:
//       :::details TITLE
//
//       content
//
//       :::
//
//   (b) inline marker — opener immediately followed by content (no blank
//       line). The first text node of the opening paragraph contains both
//       the marker and the first line of body content separated by `\n`.
//       Same pattern for the closer attached to the end of a block.
//
// Strategy: walk root.children once. When an opener is detected on a
// paragraph, split that paragraph into (HTML open) + (residual paragraph
// for any content on the same line). Then scan forward for the closer,
// also splitting the closer's host paragraph. Finally splice in HTML
// open + inner nodes + HTML close.
//
// Inner content is left as MDAST so remark-rehype renders it normally.

const OPENER_RE = /^:::details\s+([^\n]+?)\s*(?:\n([\s\S]*))?$/;
const CLOSER_RE = /^([\s\S]*?)\n?:::\s*$/;

function escape(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function htmlNode(value) {
  return { type: 'html', value };
}

function paragraphFromText(text) {
  return {
    type: 'paragraph',
    children: [{ type: 'text', value: text }],
  };
}

// Look at a paragraph node; return { title, residual } if it opens a
// :::details block, else null. `residual` is the text on the same
// paragraph that follows the opener line (often empty).
function detectOpener(node) {
  if (node.type !== 'paragraph') return null;
  const first = node.children?.[0];
  if (!first || first.type !== 'text') return null;
  const m = first.value.match(OPENER_RE);
  if (!m) return null;
  return { title: m[1].trim(), residual: m[2] ?? '' };
}

// Look at a paragraph node; return { residual } if it contains a closer
// at the end. residual is whatever text precedes the closer on this
// paragraph (often empty).
function detectCloser(node) {
  if (node.type !== 'paragraph') return null;
  const first = node.children?.[0];
  if (!first || first.type !== 'text') return null;
  const v = first.value;
  // Pure ":::" paragraph
  if (v.trim() === ':::') return { residual: '' };
  // Closer at end after some text on the same paragraph
  const m = v.match(CLOSER_RE);
  if (!m) return null;
  return { residual: m[1] };
}

export function remarkDetails() {
  return (tree) => {
    const out = [];
    let i = 0;
    while (i < tree.children.length) {
      const node = tree.children[i];
      const opener = detectOpener(node);
      if (!opener) {
        out.push(node);
        i++;
        continue;
      }

      // Find closer.
      let j = i + 1;
      let closer = null;
      while (j < tree.children.length) {
        const c = detectCloser(tree.children[j]);
        if (c) { closer = c; break; }
        j++;
      }
      if (!closer) {
        // Unterminated — leave the opener untouched.
        out.push(node);
        i++;
        continue;
      }

      // Push open tag.
      out.push(htmlNode(`<details class="details-panel"><summary>${escape(opener.title)}</summary><div class="details-body">`));
      // If opener paragraph had residual body content on the same line,
      // emit it as its own paragraph.
      if (opener.residual.trim()) {
        out.push(paragraphFromText(opener.residual));
      }
      // Push everything strictly between opener and closer.
      for (let k = i + 1; k < j; k++) out.push(tree.children[k]);
      // If closer paragraph had residual text BEFORE the ::: on the
      // same paragraph, emit it.
      if (closer.residual.trim()) {
        out.push(paragraphFromText(closer.residual));
      }
      // Push close tag.
      out.push(htmlNode('</div></details>'));

      i = j + 1;
    }
    tree.children = out;
  };
}
