// rehype plugin: wrap shell code blocks in macOS-styled terminal chrome.
// Detects <pre> elements with data-language=bash|shell|sh|zsh and wraps
// them in <div class="terminal"> with a 3-dot header. Pairs adjacent
// shell + plain output blocks into one terminal so the command + its
// output read as a single unit (mirrors the old Flutter TerminalBlock).

const SHELL_LANGS = new Set(['bash', 'shell', 'sh', 'zsh']);

function isPre(node) {
  return node && node.type === 'element' && node.tagName === 'pre';
}
function preLang(node) {
  if (!isPre(node)) return null;
  // Astro/rehype uses camelCase: data-language -> dataLanguage
  return node.properties?.dataLanguage ?? null;
}
function isShellPre(node) {
  const lang = preLang(node);
  return lang != null && SHELL_LANGS.has(lang);
}
function isOutputPre(node) {
  const lang = preLang(node);
  // Astro emits "plaintext" for unspecified-language fenced blocks.
  return lang === 'plaintext' || lang === '' || lang == null;
}

function header() {
  const dot = (cls) => ({
    type: 'element',
    tagName: 'span',
    properties: { className: ['terminal-dot', cls] },
    children: [],
  });
  return {
    type: 'element',
    tagName: 'div',
    properties: { className: ['terminal-header'] },
    children: [
      dot('r'),
      dot('y'),
      dot('g'),
      {
        type: 'element',
        tagName: 'span',
        properties: { className: ['terminal-label'] },
        children: [{ type: 'text', value: 'bash' }],
      },
    ],
  };
}

function wrap(bodyChildren) {
  return {
    type: 'element',
    tagName: 'div',
    properties: { className: ['terminal'] },
    children: [
      header(),
      {
        type: 'element',
        tagName: 'div',
        properties: { className: ['terminal-body'] },
        children: bodyChildren,
      },
    ],
  };
}

// Walk every parent in the tree and rewrite its children list. Skip any
// parent that's already inside our own terminal body so we don't recurse
// into the wrappers we just created.
function transformChildren(parent) {
  if (!parent || !Array.isArray(parent.children)) return;
  const out = [];
  let i = 0;
  while (i < parent.children.length) {
    const child = parent.children[i];
    if (isShellPre(child)) {
      // Pair with following plain block if present.
      const body = [child];
      const next = parent.children[i + 1];
      if (next && isOutputPre(next)) {
        body.push(next);
        i++;
      }
      out.push(wrap(body));
      i++;
    } else {
      out.push(child);
      i++;
    }
  }
  parent.children = out;

  // Recurse into surviving non-terminal children.
  for (const child of parent.children) {
    if (child.type !== 'element') continue;
    const cls = child.properties?.className;
    const isTerminalWrapper =
      Array.isArray(cls) && (cls.includes('terminal') || cls.includes('terminal-body'));
    if (isTerminalWrapper) continue;
    transformChildren(child);
  }
}

export function rehypeTerminal() {
  return (tree) => transformChildren(tree);
}
