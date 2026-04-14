import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";

/**
 * Self-assessment quiz webview panel.
 * Connects to the running rocky server (rocky.serverUrl setting) to record reviews.
 * Falls back to pure client-side self-assessment with instructions to sync later.
 */
export class QuizPanel {
  public static currentPanel: QuizPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private disposed = false;

  static createOrShow(
    extensionUri: vscode.Uri,
    dataProvider: RockyDataProvider,
    filter: "weak" | "all" | string // "all", "weak", or a repo name
  ): void {
    const column = vscode.window.activeTextEditor?.viewColumn ?? vscode.ViewColumn.Two;

    if (QuizPanel.currentPanel) {
      QuizPanel.currentPanel.panel.reveal(column);
      QuizPanel.currentPanel.update(dataProvider, filter);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "rockyQuiz",
      "Rocky: Quiz",
      column,
      { enableScripts: true, retainContextWhenHidden: true }
    );

    QuizPanel.currentPanel = new QuizPanel(panel, dataProvider, filter);
  }

  private constructor(
    panel: vscode.WebviewPanel,
    dataProvider: RockyDataProvider,
    filter: string
  ) {
    this.panel = panel;
    this.panel.onDidDispose(() => {
      this.disposed = true;
      QuizPanel.currentPanel = undefined;
    });
    this.update(dataProvider, filter);
  }

  private update(dataProvider: RockyDataProvider, filter: string): void {
    if (this.disposed) return;
    const serverUrl = dataProvider.getServerUrl();
    const pkg = dataProvider.getPkgData();

    let nodes = pkg?.nodes ?? [];
    if (filter === "weak") {
      nodes = nodes.filter((n) => n.retrievability < 0.5).sort((a, b) => a.retrievability - b.retrievability).slice(0, 20);
    } else if (filter !== "all") {
      nodes = nodes.filter((n) => n.repo === filter).sort((a, b) => a.retrievability - b.retrievability).slice(0, 20);
    }

    this.panel.title = `Rocky: Quiz${filter === "weak" ? " (Weakest)" : filter !== "all" ? ` — ${filter}` : ""}`;
    this.panel.webview.html = buildQuizHtml(nodes, serverUrl);
  }
}

interface QuizNode {
  name: string;
  classification: string;
  retrievability: number;
  canonical_question: string | null;
  canonical_answer: string | null;
  repo: string | null;
}

function buildQuizHtml(nodes: QuizNode[], serverUrl: string | null): string {
  const nodesJson = JSON.stringify(nodes);
  const serverUrlJson = JSON.stringify(serverUrl ?? "");

  return /* html */`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<style>
  *{box-sizing:border-box;margin:0;padding:0}
  body{font-family:var(--vscode-font-family);color:var(--vscode-foreground);background:var(--vscode-editor-background);padding:24px;max-width:600px;margin:0 auto}
  h1{font-size:16px;margin-bottom:6px}
  .sub{font-size:12px;color:var(--vscode-descriptionForeground);margin-bottom:20px}
  .progress-bar{height:3px;background:var(--vscode-input-background);border-radius:2px;overflow:hidden;margin-bottom:16px}
  .progress-fill{height:100%;background:var(--vscode-button-background);border-radius:2px;transition:width .3s}
  .meta{display:flex;gap:8px;font-size:11px;margin-bottom:10px}
  .meta span{background:var(--vscode-badge-background);color:var(--vscode-badge-foreground);padding:2px 7px;border-radius:10px}
  .question{font-size:15px;line-height:1.5;margin-bottom:14px;color:var(--vscode-editor-foreground)}
  .clue-btn{background:none;border:none;color:var(--vscode-textLink-foreground);cursor:pointer;font-size:12px;text-decoration:underline;margin-bottom:10px;display:block}
  .clue{font-size:12px;font-style:italic;padding:8px;background:var(--vscode-input-background);border-radius:4px;margin-bottom:14px;display:none}
  .assess-grid{display:grid;grid-template-columns:repeat(4,1fr);gap:8px;margin-bottom:16px}
  .assess-btn{padding:10px 4px;border-radius:6px;border:1px solid var(--vscode-button-border,#555);background:var(--vscode-input-background);color:var(--vscode-foreground);cursor:pointer;font-size:11px;font-weight:600;text-align:center;transition:all .15s}
  .assess-btn:hover{background:var(--vscode-button-hoverBackground);color:var(--vscode-button-foreground)}
  .score-icon{font-size:18px;display:block;margin-bottom:3px}
  .feedback{padding:10px;background:var(--vscode-input-background);border-radius:4px;font-size:13px;line-height:1.5;margin-bottom:14px}
  .next-btn{padding:7px 20px;border-radius:4px;border:none;background:var(--vscode-button-background);color:var(--vscode-button-foreground);cursor:pointer;font-size:13px;font-weight:600}
  .next-btn:hover{background:var(--vscode-button-hoverBackground)}
  .summary{text-align:center;padding:20px 0;display:none}
  .summary h2{font-size:20px;margin-bottom:16px}
  .sum-stats{display:flex;gap:24px;justify-content:center;margin-bottom:20px;font-size:13px}
  .sum-stat b{display:block;font-size:22px;color:var(--vscode-button-background)}
  .no-topics{padding:20px;text-align:center;color:var(--vscode-descriptionForeground)}
  .server-note{font-size:11px;color:var(--vscode-descriptionForeground);margin-top:12px;padding:8px;border:1px solid var(--vscode-input-border);border-radius:4px}
</style>
</head>
<body>
<h1>Rocky Quiz</h1>
<p class="sub" id="sub-text"></p>

<div id="quiz-area">
  <div class="progress-bar"><div class="progress-fill" id="progress"></div></div>
  <div class="meta" id="meta"></div>
  <p class="question" id="question"></p>
  <button class="clue-btn" id="clue-btn" onclick="toggleClue()" style="display:none">Show clue</button>
  <div class="clue" id="clue"></div>
  <div class="assess-grid" id="assess-grid">
    <button class="assess-btn" onclick="assess(0.15)"><span class="score-icon">✗</span>Don't know</button>
    <button class="assess-btn" onclick="assess(0.5)"><span class="score-icon">~</span>Partly</button>
    <button class="assess-btn" onclick="assess(0.85)"><span class="score-icon">✓</span>Know it</button>
    <button class="assess-btn" onclick="assess(1.0)"><span class="score-icon">★</span>Cold recall</button>
  </div>
  <div class="feedback" id="feedback" style="display:none"></div>
  <button class="next-btn" id="next-btn" onclick="next()" style="display:none">Next →</button>
</div>

<div class="summary" id="summary">
  <h2>Session complete</h2>
  <div class="sum-stats" id="sum-stats"></div>
  <button class="next-btn" onclick="restart()">Quiz again</button>
</div>

<div class="server-note" id="server-note" style="display:none"></div>

<script>
const NODES = ${nodesJson};
const SERVER_URL = ${JSON.stringify(serverUrl ?? "")};
let current = 0;
let scores = [];

function init() {
  if (NODES.length === 0) {
    document.getElementById('quiz-area').innerHTML =
      '<div class="no-topics">No topics to quiz. Run <code>rocky view</code> or check your pkg.json.</div>';
    return;
  }
  document.getElementById('sub-text').textContent = NODES.length + ' topics to review';
  if (!SERVER_URL) {
    const note = document.getElementById('server-note');
    note.textContent = 'Tip: Set rocky.serverUrl to your running rocky server to record reviews to the database.';
    note.style.display = 'block';
  }
  showQuestion();
}

function showQuestion() {
  const n = NODES[current];
  const total = NODES.length;
  document.getElementById('progress').style.width = (current/total*100)+'%';
  document.getElementById('meta').innerHTML =
    '<span>'+(n.classification||'Topic')+'</span>' +
    '<span>'+(current+1)+' / '+total+'</span>' +
    '<span>'+(Math.round(n.retrievability*100))+'% R</span>';
  document.getElementById('question').textContent =
    n.canonical_question || ('What do you know about: ' + n.name + '?');

  const clueDiv = document.getElementById('clue');
  const clueBtn = document.getElementById('clue-btn');
  // Use canonical_answer as a clue hint
  clueDiv.textContent = n.canonical_answer ? 'Hint: ' + n.canonical_answer.slice(0, 120) + '…' : '';
  clueDiv.style.display = 'none';
  clueBtn.style.display = n.canonical_answer ? 'block' : 'none';
  clueBtn.textContent = 'Show clue';

  document.getElementById('assess-grid').style.display = 'grid';
  document.getElementById('feedback').style.display = 'none';
  document.getElementById('next-btn').style.display = 'none';
}

function toggleClue() {
  const c = document.getElementById('clue');
  const showing = c.style.display === 'block';
  c.style.display = showing ? 'none' : 'block';
  document.getElementById('clue-btn').textContent = showing ? 'Show clue' : 'Hide clue';
}

async function assess(score) {
  const n = NODES[current];
  scores.push(score);
  document.getElementById('assess-grid').style.display = 'none';

  // Try to record via server
  if (SERVER_URL) {
    try {
      await fetch(SERVER_URL + '/api/quiz/assess', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({node_id: n.name, score, question: n.canonical_question || ('What do you know about: ' + n.name + '?')})
      });
    } catch(e) {
      // server not available — score recorded locally only
    }
  }

  const label = score >= 0.9 ? '★ Cold recall' : score >= 0.7 ? '✓ Know it' : score >= 0.4 ? '~ Partly' : '✗ Don\'t know';
  const color = score >= 0.7 ? '#4caf50' : score >= 0.4 ? '#ff9800' : '#f44336';
  const fb = document.getElementById('feedback');
  fb.innerHTML = '<span style="color:'+color+';font-weight:600">'+label+'</span>' +
    (n.canonical_answer ? '<br><br><b>Reference answer:</b><br>' + n.canonical_answer : '');
  fb.style.display = 'block';
  document.getElementById('next-btn').style.display = 'inline-block';
}

function next() {
  current++;
  if (current >= NODES.length) {
    showSummary();
  } else {
    showQuestion();
  }
}

function showSummary() {
  document.getElementById('quiz-area').style.display = 'none';
  const avg = scores.reduce((a,b)=>a+b,0)/(scores.length||1);
  const strong = scores.filter(s=>s>=0.7).length;
  document.getElementById('sum-stats').innerHTML =
    '<div class="sum-stat"><b>'+scores.length+'</b>reviewed</div>' +
    '<div class="sum-stat"><b>'+Math.round(avg*100)+'%</b>avg score</div>' +
    '<div class="sum-stat"><b>'+strong+'</b>solid</div>';
  document.getElementById('summary').style.display = 'block';
}

function restart() {
  current = 0; scores = [];
  document.getElementById('summary').style.display = 'none';
  document.getElementById('quiz-area').style.display = 'block';
  showQuestion();
}

init();
</script>
</body>
</html>`;
}
