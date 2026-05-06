import * as vscode from "vscode";
import { RockyDataProvider } from "./rockyDataProvider";
import { recallNow } from "./extension";

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
      nodes = nodes.filter((n) => recallNow(n) < 0.6).sort((a, b) => recallNow(a) - recallNow(b)).slice(0, 20);
    } else if (filter !== "all") {
      nodes = nodes.filter((n) => n.repo === filter).sort((a, b) => recallNow(a) - recallNow(b)).slice(0, 20);
    }

    this.panel.title = `Rocky: Quiz${filter === "weak" ? " (Weakest)" : filter !== "all" ? ` — ${filter}` : ""}`;
    this.panel.webview.html = buildQuizHtml(nodes, serverUrl);
  }
}

interface QuizNode {
  id: string;
  topic: string;
  domain: string;
  retrievability: number;
  canonical_question?: string;
  canonical_answer?: string;
  repo?: string;
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
  .answer-row{display:flex;gap:8px;align-items:stretch;margin-bottom:6px}
  .answer-row textarea{flex:1;resize:vertical;min-height:54px;padding:8px;border-radius:4px;border:1px solid var(--vscode-input-border);background:var(--vscode-input-background);color:var(--vscode-input-foreground);font-family:inherit;font-size:13px}
  .mic-btn{width:42px;border-radius:4px;border:1px solid var(--vscode-input-border);background:var(--vscode-input-background);color:var(--vscode-foreground);cursor:pointer;font-size:18px;user-select:none;-webkit-user-select:none}
  .mic-btn:hover{background:var(--vscode-button-hoverBackground)}
  .mic-btn.recording{background:#c0392b;color:#fff;border-color:#c0392b}
  .mic-btn.transcribing{background:var(--vscode-button-background);color:var(--vscode-button-foreground);border-color:transparent;cursor:progress}
  .mic-hint{font-size:11px;color:var(--vscode-descriptionForeground);min-height:14px;margin-bottom:10px}
  .mic-hint.error{color:var(--vscode-errorForeground)}
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
  <div class="answer-row">
    <textarea id="my-answer" placeholder="Speak or type your answer (optional — for your own articulation)…"></textarea>
    <button class="mic-btn" id="mic-btn" title="Hold to speak (release to transcribe)">🎤</button>
  </div>
  <div class="mic-hint" id="mic-hint"></div>
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
    '<span>'+(n.domain||'Topic')+'</span>' +
    '<span>'+(current+1)+' / '+total+'</span>' +
    '<span>'+(Math.round(n.retrievability*100))+'% R</span>';
  document.getElementById('question').textContent =
    n.canonical_question || ('What do you know about: ' + n.topic + '?');

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

  // Reset the user-answer scratchpad between questions.
  const ans = document.getElementById('my-answer');
  if (ans) ans.value = '';
  const hint = document.getElementById('mic-hint');
  if (hint) { hint.textContent = ''; hint.classList.remove('error'); }
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
        body: JSON.stringify({node_id: n.id, score, question: n.canonical_question || ('What do you know about: ' + n.topic + '?')})
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

// ══════════════════════════════════════════════════════════
// VOICE: push-to-hold mic → /api/transcribe (whisper-cpp)
// Mirrors the proven pattern in src/app.html: capture mono Float32 at 16 kHz,
// encode 16-bit PCM WAV in-browser, POST to the rocky server.
// Requires rocky.serverUrl (otherwise the mic button hides itself).
// ══════════════════════════════════════════════════════════
const micState = { stream: null, ctx: null, source: null, processor: null, buffers: [], sampleRate: 16000 };

function setMicHint(text, isError) {
  const hint = document.getElementById('mic-hint');
  if (!hint) return;
  hint.textContent = text || '';
  hint.classList.toggle('error', !!isError);
}

async function startMic(ev) {
  if (ev) ev.preventDefault();
  if (!SERVER_URL) { setMicHint('Voice needs rocky.serverUrl set (run \`rocky view\`).', true); return; }
  const btn = document.getElementById('mic-btn');
  if (!btn || btn.classList.contains('recording') || btn.classList.contains('transcribing')) return;
  setMicHint('');
  try {
    micState.stream = await navigator.mediaDevices.getUserMedia({
      audio: { channelCount: 1, sampleRate: 16000, echoCancellation: true, noiseSuppression: true }
    });
  } catch (e) {
    setMicHint('Mic blocked — grant microphone permission and try again.', true);
    return;
  }
  micState.ctx = new (window.AudioContext || window.webkitAudioContext)({ sampleRate: 16000 });
  micState.sampleRate = micState.ctx.sampleRate;
  micState.source = micState.ctx.createMediaStreamSource(micState.stream);
  micState.processor = micState.ctx.createScriptProcessor(4096, 1, 1);
  micState.buffers = [];
  micState.processor.onaudioprocess = (e) => {
    micState.buffers.push(new Float32Array(e.inputBuffer.getChannelData(0)));
  };
  micState.source.connect(micState.processor);
  micState.processor.connect(micState.ctx.destination);
  btn.classList.add('recording');
  setMicHint('Listening… release to transcribe');
}

async function stopMic(ev) {
  if (ev) ev.preventDefault();
  const btn = document.getElementById('mic-btn');
  if (!btn || !btn.classList.contains('recording')) return;
  btn.classList.remove('recording');
  btn.classList.add('transcribing');
  setMicHint('Transcribing…');

  try { micState.processor.disconnect(); } catch {}
  try { micState.source.disconnect(); } catch {}
  try { micState.stream.getTracks().forEach(t => t.stop()); } catch {}
  try { await micState.ctx.close(); } catch {}

  const wav = encodeWAV(micState.buffers, micState.sampleRate, 16000);
  micState.buffers = [];
  if (wav.byteLength <= 44) {
    btn.classList.remove('transcribing');
    setMicHint('Nothing recorded — hold the button while speaking.', true);
    return;
  }

  try {
    const r = await fetch(SERVER_URL + '/api/transcribe', {
      method: 'POST',
      headers: { 'Content-Type': 'audio/wav' },
      body: wav,
    });
    if (!r.ok) {
      if (r.status === 502) setMicHint('whisper-cli not installed. Run scripts/install-whisper.sh.', true);
      else if (r.status === 503) setMicHint('Voice is disabled in your rocky config.', true);
      else setMicHint('Transcribe failed (' + r.status + ')', true);
      return;
    }
    const data = await r.json();
    const transcript = (data && data.transcript) || '';
    insertAtCursor(document.getElementById('my-answer'), transcript);
    setMicHint('');
  } catch (e) {
    setMicHint('Network error — is \`rocky view\` still running?', true);
  } finally {
    btn.classList.remove('transcribing');
  }
}

function insertAtCursor(textarea, text) {
  if (!text || !textarea) return;
  const start = textarea.selectionStart || textarea.value.length;
  const end = textarea.selectionEnd || textarea.value.length;
  const before = textarea.value.slice(0, start);
  const after = textarea.value.slice(end);
  const sep = before && !before.endsWith(' ') ? ' ' : '';
  textarea.value = before + sep + text + after;
  textarea.focus();
}

function encodeWAV(chunks, srcRate, dstRate) {
  let totalSamples = 0;
  for (const c of chunks) totalSamples += c.length;
  const merged = new Float32Array(totalSamples);
  let offset = 0;
  for (const c of chunks) { merged.set(c, offset); offset += c.length; }
  const resampled = (srcRate === dstRate) ? merged : resampleLinear(merged, srcRate, dstRate);
  const numSamples = resampled.length;
  const byteLength = 44 + numSamples * 2;
  const buf = new ArrayBuffer(byteLength);
  const view = new DataView(buf);
  const writeStr = (off, s) => { for (let i = 0; i < s.length; i++) view.setUint8(off + i, s.charCodeAt(i)); };
  writeStr(0, 'RIFF');
  view.setUint32(4, byteLength - 8, true);
  writeStr(8, 'WAVE');
  writeStr(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, dstRate, true);
  view.setUint32(28, dstRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  writeStr(36, 'data');
  view.setUint32(40, numSamples * 2, true);
  let p = 44;
  for (let i = 0; i < numSamples; i++) {
    const s = Math.max(-1, Math.min(1, resampled[i]));
    view.setInt16(p, s < 0 ? s * 0x8000 : s * 0x7FFF, true);
    p += 2;
  }
  return buf;
}

function resampleLinear(input, srcRate, dstRate) {
  const ratio = srcRate / dstRate;
  const outLen = Math.floor(input.length / ratio);
  const out = new Float32Array(outLen);
  for (let i = 0; i < outLen; i++) {
    const srcIdx = i * ratio;
    const lo = Math.floor(srcIdx);
    const hi = Math.min(lo + 1, input.length - 1);
    const frac = srcIdx - lo;
    out[i] = input[lo] * (1 - frac) + input[hi] * frac;
  }
  return out;
}

(function bindMic() {
  const btn = document.getElementById('mic-btn');
  if (!btn) return;
  if (!SERVER_URL) {
    btn.disabled = true;
    btn.title = 'Voice requires rocky.serverUrl (run \`rocky view\`).';
    btn.style.opacity = '0.4';
    btn.style.cursor = 'not-allowed';
    return;
  }
  btn.addEventListener('mousedown', startMic);
  btn.addEventListener('mouseup', stopMic);
  btn.addEventListener('mouseleave', stopMic);
  btn.addEventListener('touchstart', startMic, { passive: false });
  btn.addEventListener('touchend', stopMic);
})();

init();
</script>
</body>
</html>`;
}
