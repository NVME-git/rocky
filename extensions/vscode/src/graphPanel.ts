import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import type { PkgData } from "./extension";

/**
 * Manages the interactive knowledge-graph webview panel.
 *
 * Uses a D3-style force-directed layout rendered entirely in the webview.
 * Supports both "global" (all topics) and "local" (current workspace repo) scopes.
 */
export class GraphPanel {
  public static currentPanel: GraphPanel | undefined;

  private readonly panel: vscode.WebviewPanel;
  private readonly dataProvider: RockyDataProvider;
  private readonly scope: "global" | "local";
  private disposed = false;

  static createOrShow(
    extensionUri: vscode.Uri,
    dataProvider: RockyDataProvider,
    scope: "global" | "local"
  ): void {
    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    if (GraphPanel.currentPanel) {
      GraphPanel.currentPanel.panel.reveal(column);
      GraphPanel.currentPanel.update(scope);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "rockyGraph",
      `Rocky: ${scope === "global" ? "Global" : "Local"} Knowledge Graph`,
      column ?? vscode.ViewColumn.One,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
      }
    );

    GraphPanel.currentPanel = new GraphPanel(panel, dataProvider, scope);
  }

  private constructor(
    panel: vscode.WebviewPanel,
    dataProvider: RockyDataProvider,
    scope: "global" | "local"
  ) {
    this.panel = panel;
    this.dataProvider = dataProvider;
    this.scope = scope;

    this.update(scope);

    this.panel.onDidDispose(() => {
      this.disposed = true;
      GraphPanel.currentPanel = undefined;
    });
  }

  private update(scope: "global" | "local"): void {
    if (this.disposed) {
      return;
    }

    const pkg = this.dataProvider.getPkgData();
    if (!pkg) {
      this.panel.webview.html = `<html><body><p>No Rocky data found. Run <code>rocky backup</code> to generate pkg.json.</p></body></html>`;
      return;
    }

    let data: PkgData;
    if (scope === "local") {
      const localNodes = this.dataProvider.getLocalNodes();
      const localNames = new Set(localNodes.map((n) => n.name));
      data = {
        nodes: localNodes,
        edges: pkg.edges.filter(
          (e) => localNames.has(e.source) && localNames.has(e.target)
        ),
      };
    } else {
      data = pkg;
    }

    this.panel.title = `Rocky: ${scope === "global" ? "Global" : "Local"} Knowledge Graph`;
    this.panel.webview.html = buildGraphHtml(data);
  }
}

function buildGraphHtml(data: PkgData): string {
  const dataJson = JSON.stringify(data);

  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Rocky Knowledge Graph</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body { width: 100%; height: 100%; overflow: hidden;
               background: var(--vscode-editor-background, #1e1e1e);
               color: var(--vscode-editor-foreground, #d4d4d4); }
  svg { width: 100%; height: 100%; }
  .link { stroke: #555; stroke-opacity: 0.6; }
  .node circle { stroke: #fff; stroke-width: 1.5px; cursor: pointer; }
  .node text { font-size: 10px; fill: var(--vscode-editor-foreground, #d4d4d4); pointer-events: none; }
  .tooltip { position: absolute; padding: 6px 10px; border-radius: 4px; font-size: 12px;
             background: var(--vscode-editorHoverWidget-background, #252526);
             border: 1px solid var(--vscode-editorHoverWidget-border, #454545);
             color: var(--vscode-editorHoverWidget-foreground, #d4d4d4);
             pointer-events: none; opacity: 0; transition: opacity 0.15s; }
  .legend { position: absolute; top: 10px; right: 10px; font-size: 11px; line-height: 1.6; }
  .legend span { display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 4px; vertical-align: middle; }
</style>
</head>
<body>
<div class="tooltip" id="tooltip"></div>
<div class="legend" id="legend"></div>
<svg id="graph"></svg>
<script>
(function() {
  const data = ${dataJson};
  const width = document.body.clientWidth;
  const height = document.body.clientHeight;

  const domainColors = {
    Language: "#e06c75", Database: "#61afef", Auth: "#c678dd",
    API: "#98c379", Frontend: "#e5c07b", DevOps: "#56b6c2",
    Architecture: "#d19a66", Performance: "#be5046", Security: "#f44747",
    Testing: "#c3e88d", Tooling: "#82aaff", Data: "#ffcb6b", Other: "#abb2bf"
  };

  // Build legend
  const domains = [...new Set(data.nodes.map(n => n.classification || "Other"))].sort();
  const legendEl = document.getElementById("legend");
  legendEl.innerHTML = domains.map(d =>
    '<div><span style="background:' + (domainColors[d] || "#abb2bf") + '"></span>' + d + '</div>'
  ).join("");

  // Build SVG
  const svg = document.getElementById("graph");
  svg.setAttribute("viewBox", "0 0 " + width + " " + height);

  const ns = "http://www.w3.org/2000/svg";

  // Create edge index for fast lookup
  const nodeMap = new Map(data.nodes.map((n, i) => [n.name, i]));

  // Draw edges
  const edgeEls = data.edges.map(e => {
    const line = document.createElementNS(ns, "line");
    line.setAttribute("class", "link");
    svg.appendChild(line);
    return { el: line, source: e.source, target: e.target };
  });

  // Draw nodes
  const tooltip = document.getElementById("tooltip");
  const nodeEls = data.nodes.map(n => {
    const g = document.createElementNS(ns, "g");
    g.setAttribute("class", "node");

    const r = 4 + Math.min(n.total_reviews, 20);
    const circle = document.createElementNS(ns, "circle");
    circle.setAttribute("r", String(r));
    circle.setAttribute("fill", domainColors[n.classification] || "#abb2bf");
    circle.style.opacity = String(0.3 + n.retrievability * 0.7);
    g.appendChild(circle);

    const text = document.createElementNS(ns, "text");
    text.setAttribute("dx", String(r + 3));
    text.setAttribute("dy", "4");
    text.textContent = n.name;
    g.appendChild(text);

    g.addEventListener("mouseover", (ev) => {
      tooltip.style.opacity = "1";
      tooltip.style.left = ev.pageX + 12 + "px";
      tooltip.style.top = ev.pageY - 20 + "px";
      tooltip.innerHTML = "<strong>" + n.name + "</strong><br/>"
        + "Domain: " + (n.classification || "Other") + "<br/>"
        + "Retrievability: " + (n.retrievability * 100).toFixed(1) + "%<br/>"
        + "Reviews: " + n.total_reviews;
    });
    g.addEventListener("mouseout", () => { tooltip.style.opacity = "0"; });

    svg.appendChild(g);
    return { el: g, node: n, x: Math.random() * width, y: Math.random() * height, vx: 0, vy: 0 };
  });

  // Simple force simulation
  function tick() {
    const k = 0.01;
    // Center gravity
    nodeEls.forEach(n => {
      n.vx += (width / 2 - n.x) * k * 0.1;
      n.vy += (height / 2 - n.y) * k * 0.1;
    });

    // Repulsion between nodes
    for (let i = 0; i < nodeEls.length; i++) {
      for (let j = i + 1; j < nodeEls.length; j++) {
        let dx = nodeEls[j].x - nodeEls[i].x;
        let dy = nodeEls[j].y - nodeEls[i].y;
        let dist = Math.sqrt(dx * dx + dy * dy) || 1;
        let force = 200 / (dist * dist);
        nodeEls[i].vx -= dx * force;
        nodeEls[i].vy -= dy * force;
        nodeEls[j].vx += dx * force;
        nodeEls[j].vy += dy * force;
      }
    }

    // Edge attraction
    edgeEls.forEach(e => {
      const si = nodeMap.get(e.source);
      const ti = nodeMap.get(e.target);
      if (si === undefined || ti === undefined) return;
      const s = nodeEls[si], t = nodeEls[ti];
      let dx = t.x - s.x, dy = t.y - s.y;
      let dist = Math.sqrt(dx * dx + dy * dy) || 1;
      let force = (dist - 80) * 0.005;
      s.vx += dx / dist * force;
      s.vy += dy / dist * force;
      t.vx -= dx / dist * force;
      t.vy -= dy / dist * force;
    });

    // Apply velocity with damping
    nodeEls.forEach(n => {
      n.vx *= 0.9;
      n.vy *= 0.9;
      n.x += n.vx;
      n.y += n.vy;
      n.x = Math.max(20, Math.min(width - 20, n.x));
      n.y = Math.max(20, Math.min(height - 20, n.y));
      n.el.setAttribute("transform", "translate(" + n.x + "," + n.y + ")");
    });

    edgeEls.forEach(e => {
      const si = nodeMap.get(e.source);
      const ti = nodeMap.get(e.target);
      if (si === undefined || ti === undefined) return;
      e.el.setAttribute("x1", String(nodeEls[si].x));
      e.el.setAttribute("y1", String(nodeEls[si].y));
      e.el.setAttribute("x2", String(nodeEls[ti].x));
      e.el.setAttribute("y2", String(nodeEls[ti].y));
    });

    requestAnimationFrame(tick);
  }
  tick();

  // Drag support
  let dragging = null;
  svg.addEventListener("mousedown", (ev) => {
    const target = ev.target.closest(".node");
    if (!target) return;
    const idx = nodeEls.findIndex(n => n.el === target);
    if (idx >= 0) dragging = nodeEls[idx];
  });
  window.addEventListener("mousemove", (ev) => {
    if (!dragging) return;
    const rect = svg.getBoundingClientRect();
    dragging.x = ev.clientX - rect.left;
    dragging.y = ev.clientY - rect.top;
    dragging.vx = 0;
    dragging.vy = 0;
  });
  window.addEventListener("mouseup", () => { dragging = null; });
})();
</script>
</body>
</html>`;
}
