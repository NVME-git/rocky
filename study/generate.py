#!/usr/bin/env python3
"""
Generate Excalidraw study diagrams for Rocky's topic lifecycle.

Run:  python3 study/generate.py
Opens: any .excalidraw file in VS Code with the Excalidraw extension,
       or drag-and-drop onto excalidraw.com

Files produced:
  01_topic_lifecycle.excalidraw     — topic state machine
  02_data_model.excalidraw          — Node struct + DB schema
  03_quiz_flow.excalidraw           — quiz session decision tree
  04_discovery_sources.excalidraw   — how topics enter the PKG
  05_fsrs_model.excalidraw          — memory/decay model
  06_source_classification.excalidraw — ai_prompt vs own_code problem space
"""

import json, os, random, string

OUT = os.path.dirname(os.path.abspath(__file__))

def uid():
    return ''.join(random.choices(string.ascii_lowercase + string.digits, k=8))

# ── Primitive builders ────────────────────────────────────────────────────────

def box(x, y, w, h, text, bg, stroke, fg="#f1f5f9", size=13,
        rounded=True, stroke_style="solid", stroke_width=2):
    bid, tid = uid(), uid()
    rect = {
        "id": bid, "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0, "strokeColor": stroke, "backgroundColor": bg,
        "fillStyle": "solid", "strokeWidth": stroke_width,
        "strokeStyle": stroke_style, "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None,
        "roundness": {"type": 3} if rounded else None,
        "isDeleted": False,
        "boundElements": [{"type": "text", "id": tid}],
        "updated": 1, "link": None, "locked": False,
    }
    txt = {
        "id": tid, "type": "text",
        "x": x + 8, "y": y + 8,
        "width": w - 16, "height": h - 16,
        "angle": 0, "strokeColor": fg, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 1, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None, "roundness": None,
        "isDeleted": False, "boundElements": [],
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": size, "fontFamily": 3,
        "textAlign": "center", "verticalAlign": "middle",
        "containerId": bid, "originalText": text, "lineHeight": 1.4,
    }
    return [rect, txt], bid

def txt(x, y, text, color="#94a3b8", size=12, align="left", width=None):
    tid = uid()
    w = width or max(len(text) * 7, 60)
    return [{
        "id": tid, "type": "text",
        "x": x, "y": y, "width": w, "height": size + 8,
        "angle": 0, "strokeColor": color, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 1, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None, "roundness": None,
        "isDeleted": False, "boundElements": [],
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": size, "fontFamily": 3,
        "textAlign": align, "verticalAlign": "top",
        "containerId": None, "originalText": text, "lineHeight": 1.4,
    }], tid

def arrow(x1, y1, x2, y2, label="", color="#475569",
          dashed=False, start_id=None, end_id=None):
    aid = uid()
    elems = [{
        "id": aid, "type": "arrow",
        "x": x1, "y": y1,
        "width": abs(x2 - x1), "height": abs(y2 - y1),
        "angle": 0, "strokeColor": color, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 2,
        "strokeStyle": "dashed" if dashed else "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None, "roundness": {"type": 2},
        "isDeleted": False, "boundElements": [],
        "updated": 1, "link": None, "locked": False,
        "points": [[0, 0], [x2 - x1, y2 - y1]],
        "lastCommittedPoint": None,
        "startBinding": {"elementId": start_id, "focus": 0.0, "gap": 6} if start_id else None,
        "endBinding":   {"elementId": end_id,   "focus": 0.0, "gap": 6} if end_id   else None,
        "startArrowhead": None, "endArrowhead": "arrow",
    }]
    if label:
        mx = x1 + (x2 - x1) / 2 - len(label) * 3.5
        my = y1 + (y2 - y1) / 2 - 10
        elems += txt(mx, my, label, color="#64748b", size=11)[0]
    return elems, aid

def note(x, y, w, h, text):
    """Dashed annotation box — for the user to fill in."""
    elems, bid = box(x, y, w, h, text,
                     bg="transparent", stroke="#334155",
                     fg="#64748b", size=12,
                     stroke_style="dashed", stroke_width=1)
    return elems, bid

def save(filename, elements):
    data = {
        "type": "excalidraw",
        "version": 2,
        "source": "https://excalidraw.com",
        "elements": elements,
        "appState": {
            "gridSize": 20,
            "viewBackgroundColor": "#0a0e1a",
        },
        "files": {},
    }
    path = os.path.join(OUT, filename)
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
    print(f"  ✓ {filename}")


# ── Colours ───────────────────────────────────────────────────────────────────
KNOWN   = ("#064e3b", "#10b981")   # bg, stroke
FADING  = ("#451a03", "#f59e0b")
GAP     = ("#450a0a", "#ef4444")
QUEUED  = ("#1c1917", "#a8a29e")
TERM    = ("#1e1b4b", "#6366f1")
PROC    = ("#0f172a", "#475569")
AI      = ("#0c4a6e", "#06b6d4")
BRAND   = ("#451a03", "#f59e0b")
DARK    = ("#111827", "#374151")
HEAD    = ("#0f172a", "#1e293b")


# ── 01: Topic Lifecycle ───────────────────────────────────────────────────────
def diagram_lifecycle():
    el = []

    # Title
    e, _ = txt(40, 20, "01 · Topic Lifecycle — State Machine", color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48, "src/main.rs · src/db.rs · src/fsrs.rs", color="#475569", size=11)
    el += e

    # States
    e, not_seen = box(40, 100, 200, 60, "UNDISCOVERED\n(not in PKG, not queued)",
                      DARK[0], DARK[1], fg="#64748b", size=12)
    el += e

    e, queued = box(40, 240, 200, 60, "QUEUED\n(.rocky local DB)",
                    QUEUED[0], QUEUED[1], size=12)
    el += e

    e, gap = box(340, 240, 200, 60, "GAP\nretrievability < 0.7",
                 GAP[0], GAP[1], size=12)
    el += e

    e, fading = box(340, 360, 200, 60, "FADING\n0.7 ≤ R < 0.9",
                    FADING[0], FADING[1], size=12)
    el += e

    e, known = box(340, 480, 200, 60, "KNOWN\nretrievability ≥ 0.9",
                   KNOWN[0], KNOWN[1], size=12)
    el += e

    e, ignored = box(40, 360, 180, 50, "IGNORED\n(dismissed, not in PKG)",
                     TERM[0], TERM[1], fg="#a5b4fc", size=11)
    el += e

    e, deleted = box(640, 240, 180, 50, "DELETED\nrocky delete",
                     "#1a1a1a", "#374151", fg="#6b7280", size=11)
    el += e

    # IN PKG group label
    e, _ = txt(300, 200, "─── IN PKG ────────────────────────────", color="#1e293b", size=11)
    el += e

    # Transitions
    e, _ = arrow(140, 160, 140, 240, "rocky diff / backfill\nrocky quiz (from logs)\nrocky \"task\"",
                 color="#94a3b8", start_id=not_seen, end_id=queued)
    el += e

    e, _ = arrow(240, 270, 340, 270, "first Q&A\nscore < 0.65",
                 color="#ef4444", start_id=queued, end_id=gap)
    el += e

    e, _ = arrow(130, 300, 80, 360, "'i' ignore",
                 color="#6366f1", start_id=queued, end_id=ignored)
    el += e

    e, _ = arrow(200, 250, 200, 230, "", color="#64748b")  # skip loop (back to queued)
    el += e
    e, _ = txt(208, 228, "'Enter' skip → stays queued", color="#64748b", size=10)
    el += e

    e, _ = arrow(440, 300, 440, 360, "quiz, partial\nscore 0.3–0.65",
                 color="#f59e0b", start_id=gap, end_id=fading)
    el += e

    e, _ = arrow(440, 420, 440, 480, "quiz, good\nscore ≥ 0.65",
                 color="#10b981", start_id=fading, end_id=known)
    el += e

    e, _ = arrow(390, 300, 390, 480, "quiz, high\nscore ≥ 0.65",
                 color="#10b981", start_id=gap, end_id=known)
    el += e

    # Decay arrows (right side, going back up)
    e, _ = arrow(550, 510, 550, 390, "time decay\n(low stability)",
                 color="#f59e0b", dashed=True, start_id=known, end_id=fading)
    el += e

    e, _ = arrow(570, 390, 570, 270, "time decay\n(very low stability)",
                 color="#ef4444", dashed=True, start_id=fading, end_id=gap)
    el += e

    # Too easy shortcut
    e, _ = arrow(160, 260, 350, 500, "'e' too easy\nscore=0.75",
                 color="#10b981", start_id=queued, end_id=known)
    el += e

    # Deleted
    e, _ = arrow(540, 270, 640, 265, "rocky delete",
                 color="#374151", start_id=gap, end_id=deleted)
    el += e

    # '?' explain path (from gap)
    e, explained = box(640, 360, 200, 60, "EXPLAINED\n'?' during quiz\nscore=0.2 → added to PKG",
                       AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(540, 270, 640, 370, "'?' explain\n→ low confidence", color="#06b6d4",
                 start_id=gap, end_id=explained)
    el += e

    # Annotation areas
    e, _ = note(40, 580, 400, 80,
                "📝 ANNOTATION: Which transitions are you uncertain about?\nAdd notes here.")
    el += e
    e, _ = note(480, 580, 360, 80,
                "📝 ANNOTATION: Proposed new transitions for\nai_prompt / co_authored features?")
    el += e

    # Legend
    e, _ = txt(40, 690, "LEGEND", color="#f59e0b", size=12)
    el += e
    e, _ = box(40, 710, 100, 30, "KNOWN", KNOWN[0], KNOWN[1], size=11)
    el += e
    e, _ = box(160, 710, 100, 30, "FADING", FADING[0], FADING[1], size=11)
    el += e
    e, _ = box(280, 710, 100, 30, "GAP", GAP[0], GAP[1], size=11)
    el += e
    e, _ = box(400, 710, 120, 30, "QUEUED", QUEUED[0], QUEUED[1], size=11)
    el += e
    e, _ = txt(40, 752, "Dashed arrows = time-based decay (no user action)",
               color="#475569", size=11)
    el += e

    save("01_topic_lifecycle.excalidraw", el)


# ── 02: Data Model ────────────────────────────────────────────────────────────
def diagram_data_model():
    el = []

    e, _ = txt(40, 20, "02 · Data Model — Node struct + DB schema", color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48, "src/node.rs · src/db.rs  (struct Node, SCHEMA const, row_to_node)",
               color="#475569", size=11)
    el += e

    # Main Node box
    node_fields = (
        "Node  (src/node.rs)\n"
        "─────────────────────────────\n"
        "IDENTITY\n"
        "  id: String          sha256(topic.lower())[..16]\n"
        "  topic: String        human-readable name\n"
        "  kind: Kind           concept | pattern | implementation | domain\n"
        "  domain: String       one of 13 taxonomy domains\n"
        "\n"
        "FSRS MEMORY MODEL\n"
        "  stability: f64       how embedded — higher = decays slower\n"
        "  difficulty: f64      how hard historically (0.0–1.0)\n"
        "  last_reviewed: Date  last time user was quizzed\n"
        "  last_encountered: Date  last time topic appeared in any context\n"
        "  review_count: i64    total quiz sessions for this topic\n"
        "\n"
        "CONTENT\n"
        "  description: String  1-2 sentence summary\n"
        "  contexts: Vec<String>  task/commit messages this appeared in\n"
        "  canonical_question: String  pre-generated at backfill (or empty)\n"
        "  canonical_answer: String    ideal answer for evaluator reference\n"
        "  canonical_clue: String      short hint shown on [c]\n"
        "\n"
        "ORIGIN\n"
        "  repo: String         parsed from git remote URL\n"
        "  created_at: Date     commit date (backfill) or today (live)\n"
        "\n"
        "COMPUTED (not stored)\n"
        "  retrievability: f64  R = (1 + t / 9s)^-1  →  0.0..1.0\n"
        "  classification: str  known | fading | gap"
    )
    e, node_id = box(40, 80, 560, 520, node_fields,
                     "#0f172a", "#334155", fg="#cbd5e1", size=12, rounded=False)
    el += e

    # contexts table
    ctx_fields = (
        "contexts  (DB table)\n"
        "──────────────────────\n"
        "node_id TEXT  FK → nodes.id\n"
        "context TEXT  task or commit msg\n"
        "added_at TEXT date"
    )
    e, ctx_id = box(660, 80, 260, 110, ctx_fields,
                    "#0f172a", "#334155", fg="#94a3b8", size=12, rounded=False)
    el += e
    e, _ = arrow(600, 200, 660, 130, "1:many", color="#374151",
                 start_id=node_id, end_id=ctx_id)
    el += e

    # reviews table
    rev_fields = (
        "reviews  (DB table)\n"
        "──────────────────────────────────\n"
        "node_id TEXT    FK → nodes.id\n"
        "question TEXT   question asked\n"
        "answer TEXT     user's answer\n"
        "feedback TEXT   Rocky's feedback\n"
        "score REAL      0.0–1.0\n"
        "reviewed_at TEXT date\n"
        "\n"
        "⚠ struggle score not yet stored here\n"
        "  (planned: follow_up_count, clue_used,\n"
        "   explain_used, simpler_used)"
    )
    e, rev_id = box(660, 210, 300, 220, rev_fields,
                    "#0f172a", "#334155", fg="#94a3b8", size=12, rounded=False)
    el += e
    e, _ = arrow(600, 300, 660, 310, "1:many", color="#374151",
                 start_id=node_id, end_id=rev_id)
    el += e

    # edges table
    edge_fields = (
        "edges  (DB table)\n"
        "──────────────────────────────────────\n"
        "id TEXT            unique edge id\n"
        "source_id TEXT     FK → nodes.id\n"
        "target_id TEXT     FK → nodes.id\n"
        "kind TEXT          implies | depends_on\n"
        "                   conflicts_with | part_of\n"
        "description TEXT   why Rocky created this edge\n"
        "strength REAL      0.0–1.0\n"
        "created_at TEXT\n"
        "last_fired TEXT    last time edge triggered a question\n"
        "last_fired_session INT  session index"
    )
    e, edge_id = box(660, 450, 340, 200, edge_fields,
                     "#0f172a", "#334155", fg="#94a3b8", size=12, rounded=False)
    el += e
    e, _ = arrow(600, 400, 660, 520, "many:many", color="#374151",
                 start_id=node_id, end_id=edge_id)
    el += e

    # session KV table
    kv_fields = (
        "session  (DB table — key/value)\n"
        "────────────────────────────────\n"
        "key TEXT    PK\n"
        "value TEXT\n"
        "\n"
        "Keys used:\n"
        "  quiz_count_today\n"
        "  last_quiz_time\n"
        "  total_quizzes"
    )
    e, _ = box(40, 640, 260, 140, kv_fields,
               "#0f172a", "#334155", fg="#94a3b8", size=12, rounded=False)
    el += e

    # Migrations note
    mig = (
        "DB Migrations  (src/db.rs — open())\n"
        "─────────────────────────────────────────\n"
        "All ALTER TABLE calls are idempotent.\n"
        "Columns added over time:\n"
        "  domain, canonical_question,\n"
        "  canonical_answer, canonical_clue, repo\n"
        "\n"
        "New columns needed (planned):\n"
        "  co_authored BOOL  DEFAULT 0\n"
        "  source TEXT       DEFAULT 'own_code'"
    )
    e, _ = box(320, 640, 380, 160, mig,
               "#0c1a0f", "#1d4ed8", fg="#93c5fd", size=12, rounded=False)
    el += e

    # Annotation
    e, _ = note(660, 660, 340, 100,
                "📝 ANNOTATION: Which fields are\nmissing or should change?\nMark them here.")
    el += e

    save("02_data_model.excalidraw", el)


# ── 03: Quiz Flow ─────────────────────────────────────────────────────────────
def diagram_quiz_flow():
    el = []

    e, _ = txt(40, 20, "03 · Quiz Session — Decision Tree", color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48, "src/main.rs — run_quiz_topic()  ~line 1130",
               color="#475569", size=11)
    el += e

    # Start
    e, start = box(300, 90, 200, 50, "Topic selected\nfor quiz", PROC[0], PROC[1], size=12)
    el += e

    # Cross-concept check
    e, cc_check = box(220, 190, 360, 60,
                      "Check: cross-concept edge?\n(peer node with same edge + both recall > 0.8?)",
                      PROC[0], "#1e40af", fg="#bfdbfe", size=12)
    el += e
    e, _ = arrow(400, 140, 400, 190, "", start_id=start, end_id=cc_check)
    el += e

    # Cross-concept YES
    e, cc_q = box(640, 170, 220, 60, "generate_cross_concept_question()\n[generated — live]",
                  AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(580, 220, 640, 200, "YES\nfire edge", color="#06b6d4",
                 start_id=cc_check, end_id=cc_q)
    el += e

    # Canonical check
    e, can_check = box(220, 310, 360, 60,
                       "Check: canonical_question non-empty?",
                       PROC[0], "#334155", fg="#cbd5e1", size=12)
    el += e
    e, _ = arrow(400, 250, 400, 310, "NO edge", start_id=cc_check, end_id=can_check)
    el += e

    # Canonical YES
    e, can_q = box(640, 290, 220, 60, "use canonical_question\n[canonical — from diff]",
                   KNOWN[0], KNOWN[1], fg="#d1fae5", size=11)
    el += e
    e, _ = arrow(580, 340, 640, 320, "YES", color="#10b981",
                 start_id=can_check, end_id=can_q)
    el += e

    # Generated
    e, gen_q = box(640, 390, 220, 60, "generate_question()\n[generated — live]",
                   AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(580, 350, 640, 410, "NO", color="#06b6d4",
                 start_id=can_check, end_id=gen_q)
    el += e

    # Display question (all 3 paths lead here)
    e, display = box(220, 460, 360, 70,
                     "Display question\n+ label (canonical) or (generated)\n+ load canonical_answer, canonical_clue",
                     BRAND[0], BRAND[1], fg="#fef3c7", size=12)
    el += e
    e, _ = arrow(400, 370, 400, 460, "", start_id=can_check, end_id=display)
    el += e
    e, _ = arrow(640, 200, 400, 465, "", color="#475569",
                 start_id=cc_q, end_id=display)
    el += e
    e, _ = arrow(640, 320, 530, 470, "", color="#475569",
                 start_id=can_q, end_id=display)
    el += e
    e, _ = arrow(640, 420, 540, 480, "", color="#475569",
                 start_id=gen_q, end_id=display)
    el += e

    # Show options
    opts = (
        "Prompt options shown:\n"
        "  [e] too easy   [s] simpler   [h] harder\n"
        "  [c] clue       [?] explain   [i] ignore\n"
        "  Enter (blank)  or type answer"
    )
    e, opt_box = box(100, 580, 600, 80, opts, DARK[0], "#1e293b", fg="#94a3b8", size=12)
    el += e
    e, _ = arrow(400, 530, 400, 580, "", start_id=display, end_id=opt_box)
    el += e

    # Option branches (below options box)
    y0 = 720

    e, skip = box(40, y0, 120, 50, "Enter\n→ queue topic\n→ exit", QUEUED[0], QUEUED[1], size=11)
    el += e
    e, _ = arrow(100, 660, 100, y0, "'Enter'", color="#a8a29e", start_id=opt_box, end_id=skip)
    el += e

    e, ign = box(180, y0, 120, 50, "'i'\n→ ignored\n→ exit", TERM[0], TERM[1], fg="#a5b4fc", size=11)
    el += e
    e, _ = arrow(200, 660, 220, y0, "'i'", color="#6366f1", start_id=opt_box, end_id=ign)
    el += e

    e, easy = box(320, y0, 130, 50, "'e'\n→ score=0.75\n→ add to PKG", KNOWN[0], KNOWN[1], size=11)
    el += e
    e, _ = arrow(360, 660, 370, y0, "'e'", color="#10b981", start_id=opt_box, end_id=easy)
    el += e

    e, simpler = box(470, y0, 130, 50, "'s' / 'h'\n→ regen question\n→ loop back",
                     AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(500, 660, 510, y0, "'s'/'h'", color="#06b6d4", start_id=opt_box, end_id=simpler)
    el += e

    e, clue = box(620, y0, 130, 50, "'c'\n→ show clue\n→ loop back",
                  PROC[0], "#0e7490", fg="#cffafe", size=11)
    el += e
    e, _ = arrow(620, 660, 670, y0, "'c'", color="#0e7490", start_id=opt_box, end_id=clue)
    el += e

    e, explain = box(770, y0, 140, 50, "'?'\n→ explanation\n→ score=0.2\n→ add to PKG",
                     AI[0], "#0e7490", fg="#cffafe", size=11)
    el += e
    e, _ = arrow(680, 660, 820, y0, "'?'", color="#0e7490", start_id=opt_box, end_id=explain)
    el += e

    # Answer path
    y1 = y0 + 110
    e, ans = box(220, y1, 360, 60, "User types answer\n→ evaluate_answer(topic, q, answer, canonical_answer)",
                 PROC[0], "#334155", fg="#cbd5e1", size=12)
    el += e
    e, _ = arrow(400, 660, 400, y1, "type answer", color="#f59e0b",
                 start_id=opt_box, end_id=ans)
    el += e

    y2 = y1 + 100
    e, understood = box(100, y2, 180, 60, "score ≥ 0.65\nunderstood=true\n→ add to PKG\n→ exit",
                        KNOWN[0], KNOWN[1], size=11)
    el += e
    e, _ = arrow(300, y1 + 60, 190, y2, "understood", color="#10b981",
                 start_id=ans, end_id=understood)
    el += e

    e, followup = box(330, y2, 200, 60, "score < 0.65\nhas followup?\nquestions_asked < 3?",
                      FADING[0], FADING[1], size=11)
    el += e
    e, _ = arrow(400, y1 + 60, 420, y2, "not understood", color="#f59e0b",
                 start_id=ans, end_id=followup)
    el += e

    e, exhaust = box(570, y2, 180, 60, "exhausted\n→ generate_explanation()\n→ score=avg\n→ exit",
                     AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(530, y2 + 30, 570, y2 + 30, "NO followup\nor MAX_Q=3 hit",
                 color="#ef4444", start_id=followup, end_id=exhaust)
    el += e

    # Followup loops back
    e, _ = arrow(430, y2 + 60, 400, y0 + 60, "YES → new question",
                 color="#f59e0b", dashed=True, start_id=followup, end_id=opt_box)
    el += e

    # Annotation
    e, _ = note(780, 580, 220, 140,
                "📝 ANNOTATION:\nWhere should struggle\ntracking be inserted?\n\nMark each decision\npoint that should\nincrement a counter.")
    el += e

    save("03_quiz_flow.excalidraw", el)


# ── 04: Discovery Sources ─────────────────────────────────────────────────────
def diagram_discovery():
    el = []

    e, _ = txt(40, 20, "04 · Topic Discovery — How Topics Enter the PKG", color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48, "src/main.rs — run_diff(), run_backfill(), run_quiz()",
               color="#475569", size=11)
    el += e

    # Central PKG
    e, pkg = box(380, 360, 240, 80, "PKG\ngraph.db\n~/.rocky/graph.db",
                 BRAND[0], BRAND[1], fg="#fef3c7", size=14)
    el += e

    # Source 1: rocky diff
    diff_text = (
        "rocky diff  /  git post-commit hook\n"
        "──────────────────────────────────\n"
        "Source:   git diff (actual code)\n"
        "Trigger:  manual or post-commit\n"
        "initial_r: 0.5  (neutral)\n"
        "created_at: today\n"
        "canonical Q&A: generated at insertion\n"
        "canonical clue: generated at insertion\n"
        "edges: generated after batch\n"
        "\n"
        "⚠ Co-Authored-By trailer detectable here\n"
        "  → proposed: co_authored=true if present"
    )
    e, diff_box = box(40, 60, 300, 220, diff_text,
                      PROC[0], "#1e40af", fg="#bfdbfe", size=11)
    el += e
    e, _ = arrow(340, 200, 380, 390, "new topics\nfrom diff",
                 color="#3b82f6", start_id=diff_box, end_id=pkg)
    el += e

    # Source 2: rocky backfill
    backfill_text = (
        "rocky backfill\n"
        "──────────────────────────────────\n"
        "Source:   git log history (oldest first)\n"
        "Trigger:  manual only\n"
        "initial_r: 0.5\n"
        "created_at: COMMIT DATE (not today)\n"
        "  → key: decay starts from original date\n"
        "canonical Q&A + clue: generated per topic\n"
        "  (uses diff + commit msg + README summary)\n"
        "edges: generated in bulk at end\n"
        "repo: parsed from git remote URL\n"
        "\n"
        "--fill-clues: retroactive clue generation\n"
        "--all-authors: include other committers\n"
        "--limit N: cap at N commits"
    )
    e, backfill_box = box(40, 320, 300, 240, backfill_text,
                          PROC[0], "#065f46", fg="#a7f3d0", size=11)
    el += e
    e, _ = arrow(340, 430, 380, 410, "new topics\nwith history",
                 color="#10b981", start_id=backfill_box, end_id=pkg)
    el += e

    # Source 3: rocky "task"
    task_text = (
        "rocky \"task description\"\n"
        "──────────────────────────────────\n"
        "Source:   user's text description\n"
        "Trigger:  manual, before starting work\n"
        "initial_r: set after Q&A completes\n"
        "created_at: today\n"
        "canonical Q&A: NOT pre-generated\n"
        "  (question generated live from description)\n"
        "Topic enters PKG only AFTER Q&A passes\n"
        "\n"
        "No daily limits — user-initiated\n"
        "No Co-Authored-By signal available"
    )
    e, task_box = box(680, 60, 300, 220, task_text,
                      PROC[0], "#7c3aed", fg="#ddd6fe", size=11)
    el += e
    e, _ = arrow(680, 200, 620, 380, "topics after\nQ&A passes",
                 color="#8b5cf6", start_id=task_box, end_id=pkg)
    el += e

    # Source 4: rocky quiz (from prompt logs)
    quiz_text = (
        "rocky quiz  (from .rocky prompt log)\n"
        "──────────────────────────────────\n"
        "Source:   Claude Code prompts logged by hook\n"
        "Trigger:  rocky quiz (reads last 24h)\n"
        "initial_r: set after Q&A completes\n"
        "created_at: today\n"
        "canonical Q&A: NOT pre-generated\n"
        "  (live generation from topic + description)\n"
        "\n"
        "STRONGEST ai_prompt signal:\n"
        "  topic seen in your prompts but\n"
        "  never in a diff you committed\n"
        "\n"
        "Log file: ./.rocky  (SQLite, per project)\n"
        "Entries auto-deleted after 24h"
    )
    e, quiz_box = box(680, 320, 300, 260, quiz_text,
                      AI[0], AI[1], fg="#e0f2fe", size=11)
    el += e
    e, _ = arrow(680, 430, 620, 420, "new topics\nfrom prompts",
                 color="#06b6d4", start_id=quiz_box, end_id=pkg)
    el += e

    # .rocky file
    e, rocky_file = box(680, 600, 200, 80,
                        ".rocky  (SQLite)\nprompt_text\ntimestamp\nsession_id",
                        DARK[0], "#334155", fg="#64748b", size=11)
    el += e
    e, _ = arrow(750, 580, 750, 600, "hook logs", color="#334155",
                 start_id=quiz_box, end_id=rocky_file)
    el += e
    e, _ = txt(720, 688, "rocky hook → writes here", color="#334155", size=10)
    el += e

    # PKG output paths
    e, _ = txt(400, 458, "rocky export\n→ obsidian .md files",
               color="#64748b", size=11, width=200)
    el += e
    e, _ = txt(400, 500, "rocky view\n→ ~/.rocky/view.html",
               color="#64748b", size=11, width=200)
    el += e

    # Annotation
    e, _ = note(40, 600, 300, 100,
                "📝 ANNOTATION: How should co_authored\nflag be set for backfill?\n"
                "Backfill reads git log — Co-Authored-By\n"
                "trailer is in commit body. Parseable?")
    el += e

    e, _ = note(380, 600, 280, 100,
                "📝 ANNOTATION: For rocky \"task\",\n"
                "should origin be 'task_prompt'\n"
                "rather than 'own_code' or 'ai_prompt'?\n"
                "It's neither — user described intent.")
    el += e

    save("04_discovery_sources.excalidraw", el)


# ── 05: FSRS Model ────────────────────────────────────────────────────────────
def diagram_fsrs():
    el = []

    e, _ = txt(40, 20, "05 · Memory Model — FSRS (Free Spaced Repetition Scheduler)", color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48, "src/fsrs.rs  ·  src/db.rs — add_or_update()",
               color="#475569", size=11)
    el += e

    # Formula
    e, _ = box(40, 80, 560, 100,
               "Retrievability formula  (src/fsrs.rs — retrieve())\n\n"
               "R  =  ( 1  +  t / (9 × S) ) ^ -1\n\n"
               "  t = days since last_reviewed\n"
               "  S = stability (stored on node)",
               BRAND[0], BRAND[1], fg="#fef3c7", size=14)
    el += e

    # Classification thresholds
    e, _ = box(640, 80, 300, 100,
               "Classification  (src/fsrs.rs — classify())\n\n"
               "R ≥ 0.9   →  known\n"
               "0.7 ≤ R < 0.9  →  fading\n"
               "R < 0.7   →  gap",
               PROC[0], "#334155", fg="#cbd5e1", size=13)
    el += e

    # Decay examples
    decay_text = (
        "Decay examples  (t at which R = 0.5)\n"
        "─────────────────────────────────────────────────────\n"
        "Stability  →  Days until R drops to 0.5\n\n"
        "S = 2.0  (new topic, first insertion)\n"
        "    t = 9 × 2 = 18 days → R=0.5\n"
        "    known threshold (R=0.9) crossed at ~2 days\n\n"
        "S = 5.0  (reviewed twice, decent answers)\n"
        "    t = 9 × 5 = 45 days → R=0.5\n"
        "    known threshold crossed at ~5.5 days\n\n"
        "S = 10.0  (strong repeated understanding)\n"
        "    t = 9 × 10 = 90 days → R=0.5\n"
        "    stays known for ~11 days\n\n"
        "S = 999  (domain skeleton node)\n"
        "    effectively never decays (R ≈ 1.0 forever)"
    )
    e, _ = box(40, 210, 500, 290, decay_text,
               PROC[0], "#334155", fg="#cbd5e1", size=12)
    el += e

    # How stability changes
    stability_text = (
        "Stability updates  (src/db.rs — add_or_update)\n"
        "────────────────────────────────────────────────────\n"
        "Called after every quiz or encounter.\n\n"
        "On first insertion:\n"
        "  stability = 2.0  (default)\n"
        "  difficulty = 0.3  (default)\n\n"
        "On review:\n"
        "  new_s = old_s × (1 + score × 0.9)\n"
        "  difficulty = difficulty × 0.9 + score × 0.1\n\n"
        "Score sources:\n"
        "  'e' too easy        → score = 0.75\n"
        "  '?' explained       → score = 0.2\n"
        "  understood (≥0.65)  → score = eval result\n"
        "  exhausted follow-up → score = avg of attempts\n"
        "  stale reminder      → score = 0.5  (small bump)\n\n"
        "Note: stability COMPOUNDS — answer well 3×\n"
        "and a topic may not resurface for months."
    )
    e, _ = box(560, 210, 420, 290, stability_text,
               PROC[0], "#334155", fg="#cbd5e1", size=12)
    el += e

    # Initial values by source
    init_text = (
        "Initial retrievability by entry path\n"
        "────────────────────────────────────────\n"
        "rocky diff / backfill:\n"
        "  stability = 2.0, review_count = 0\n"
        "  R = 1.0 at creation (just seen)\n"
        "  but created_at may be old (backfill)\n"
        "  so R may already be < 0.9 when you first quiz\n\n"
        "rocky \"task\" / rocky quiz:\n"
        "  added AFTER Q&A — score sets stability\n"
        "  understood → stability depends on score quality\n"
        "  explained → stability = low (score=0.2)\n\n"
        "Domain skeleton nodes:\n"
        "  stability = 999  →  R = 1.0 always\n"
        "  Never quizzed, just structure"
    )
    e, _ = box(40, 530, 500, 240, init_text,
               PROC[0], "#334155", fg="#cbd5e1", size=12)
    el += e

    # Proposed: co_authored affects initial stability
    proposed_text = (
        "PROPOSED CHANGE  (not yet implemented)\n"
        "────────────────────────────────────────────\n"
        "If co_authored = true:\n"
        "  initial stability = 1.5  (instead of 2.0)\n"
        "  initial difficulty = 0.4  (slightly harder)\n"
        "\n"
        "Rationale:\n"
        "  AI-generated code = less certain the dev\n"
        "  understood it at commit time.\n"
        "  Lower starting stability → topic resurfaces\n"
        "  sooner, forcing earlier verification.\n\n"
        "📝 ANNOTATION: Is this the right signal?\n"
        "Or should the quiz scheduling be unchanged\n"
        "but the PKG display flag it differently?"
    )
    e, _ = box(560, 530, 420, 240, proposed_text,
               "#0c1a0f", "#1d4ed8", fg="#93c5fd", size=12)
    el += e

    save("05_fsrs_model.excalidraw", el)


# ── 06: Source Classification Problem ─────────────────────────────────────────
def diagram_classification():
    el = []

    e, _ = txt(40, 20, "06 · Source Classification — The ai_prompt vs own_code Problem",
               color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 48,
               "Discussion: 2026-04-10  ·  See BACKLOG.md — AI-source tagging",
               color="#475569", size=11)
    el += e

    # The broken assumption
    e, _ = box(40, 80, 680, 80,
               "BROKEN ASSUMPTION: commit authorship = developer understood the code\n\n"
               "Reality: developer may have committed AI-generated code without deeply understanding it.\n"
               "Rocky project itself: 0 lines written by hand, all commits in developer's name.",
               GAP[0], GAP[1], fg="#fee2e2", size=12)
    el += e

    # Signals — two columns
    e, _ = txt(40, 190, "Available signals", color="#f59e0b", size=14)
    el += e

    # Reliable
    reliable = (
        "RELIABLE\n"
        "──────────────────────────────────────────\n"
        "1. Co-Authored-By trailer in commit body\n"
        "   'Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>'\n"
        "   → parseable from git log --format=%B\n"
        "   → strong signal: AI explicitly co-authored\n"
        "   → present in rocky commits, copilot commits\n\n"
        "2. No-diff entry (only in prompt log, never in diff)\n"
        "   → definitively ai_prompt\n"
        "   → topic in .rocky but no matching git diff"
    )
    e, rel_box = box(40, 220, 440, 180, reliable,
                     KNOWN[0], KNOWN[1], fg="#d1fae5", size=12)
    el += e

    # Moderate
    moderate = (
        "MODERATE\n"
        "──────────────────────────────────────────\n"
        "3. Prompt log proximity\n"
        "   Prompt about topic X within ~hours of\n"
        "   a commit introducing topic X\n"
        "   → correlation, not causation\n"
        "   → requires .rocky log to predate commit\n\n"
        "4. Diff velocity\n"
        "   >500 lines / >10 files in one commit\n"
        "   → unusual for solo hand-writing\n"
        "   → weak alone, adds weight to other signals"
    )
    e, _ = box(40, 420, 440, 180, moderate,
               FADING[0], FADING[1], fg="#fef3c7", size=12)
    el += e

    # Unreliable
    unreliable = (
        "UNRELIABLE  ✗\n"
        "──────────────────────────────────────────\n"
        "✗ Commit authorship  (disproved above)\n\n"
        "✗ Commit message style\n"
        "  Claude writes 'feat:' / 'fix:' but so do\n"
        "  disciplined human devs\n\n"
        "✗ Code quality / cleanliness\n"
        "  AI writes clean code; so do good devs\n\n"
        "✗ File count / change size alone\n"
        "  Refactors can be large and hand-written"
    )
    e, _ = box(40, 620, 440, 180, unreliable,
               GAP[0], GAP[1], fg="#fee2e2", size=12)
    el += e

    # What Rocky controls
    controls = (
        "WHAT ROCKY ACTUALLY CONTROLS\n"
        "(the only fully reliable signal)\n"
        "────────────────────────────────────────────────\n"
        "Quiz history in reviews table:\n\n"
        "  clean_pass:\n"
        "    answered correctly, first attempt,\n"
        "    no [c] clue, no [?] explain, no [s] simpler\n\n"
        "  scaffolded_pass:\n"
        "    answered correctly but used scaffolding\n\n"
        "  failed:\n"
        "    exhausted follow-ups, got explanation\n\n"
        "A topic with clean_pass = true is understood\n"
        "regardless of who wrote the original code.\n\n"
        "This is what the struggle score measures.\n"
        "It is independent of code origin entirely."
    )
    e, ctrl_box = box(520, 220, 440, 300, controls,
                      KNOWN[0], "#10b981", fg="#d1fae5", size=12)
    el += e

    # Proposed approach
    proposed = (
        "PROPOSED APPROACH  (two-track)\n"
        "────────────────────────────────────────────────\n"
        "Track 1 — Code origin flag (best-effort)\n"
        "  co_authored: bool\n"
        "    = Co-Authored-By found in commit body\n"
        "  source: 'own_code' | 'ai_prompt' | 'task_prompt'\n"
        "    own_code   = diff authored by dev, no AI co-author\n"
        "    ai_prompt  = only in prompt log, no matching diff\n"
        "    task_prompt = from rocky \"task\" pre-task prompt\n"
        "  Effect: lower initial stability for co_authored\n\n"
        "Track 2 — Independence score (authoritative)\n"
        "  clean_pass_count: int  (no scaffolding used)\n"
        "  scaffolded_pass_count: int\n"
        "  Displayed in rocky stats / rocky ls\n"
        "  This is the real measure of competence"
    )
    e, _ = box(520, 540, 440, 260, proposed,
               PROC[0], "#1e40af", fg="#bfdbfe", size=12)
    el += e

    # Open questions annotation area
    e, _ = note(40, 820, 920, 140,
                "📝 OPEN QUESTIONS — annotate with your decisions:\n\n"
                "1. Should co_authored lower initial stability, or just flag it in the UI?\n"
                "2. What's the minimum clean_pass_count before a topic is considered 'independently understood'?\n"
                "3. For topics predating this feature — can we infer source from git log Co-Authored-By retroactively?\n"
                "4. Should task_prompt be treated the same as ai_prompt, or differently?\n"
                "5. When prompted + committed (most common case) — does co_authored flag = true win, or does diff presence win?")
    el += e

    save("06_source_classification.excalidraw", el)


# ── 07: Rich-Context Pipeline ─────────────────────────────────────────────────
def diagram_rich_context_pipeline():
    el = []
    e, _ = txt(40, 20, "07 · Rich-Context Pipeline — explore → post-commit → session-end",
               color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 52, "src/main.rs · src/transcript.rs", color="#475569", size=11)
    el += e

    stages = (
        "STAGE 1 — rocky explore  (run once per project)\n"
        "─────────────────────────────────────────────────────────\n"
        "• Reads git log, README, Cargo.toml / package.json\n"
        "• Stores project_context summary in DB\n"
        "• Seeds domain taxonomy skeleton\n"
        "• Extracts initial topics from history\n"
        "• rocky explore --show  →  print stored context\n"
        "• rocky explore --force →  re-run even if context exists"
    )
    e, _ = box(40, 80, 540, 180, stages, "#0c4a6e", "#06b6d4", fg="#e0f2fe", size=12)
    el += e

    queue = (
        "STAGE 2 — rocky post-commit  (git hook, per commit)\n"
        "─────────────────────────────────────────────────────────\n"
        "• No LLM call — takes <5 ms\n"
        "• Appends HEAD SHA to .rocky commit queue\n"
        "• Multiple commits accumulate between sessions\n"
        "• Hook installed by: rocky hook install --mode queue"
    )
    e, _ = box(40, 280, 540, 140, queue, "#1c1917", "#a8a29e", fg="#e2e8f0", size=12)
    el += e

    session = (
        "STAGE 3 — rocky session-end  (Claude Code Stop hook)\n"
        "─────────────────────────────────────────────────────────\n"
        "1. Drain commit queue → git diff each SHA\n"
        "2. Read last N hours of transcript\n"
        "   (~/.claude/projects/<repo>/*.jsonl)\n"
        "3. One LLM call: diffs + transcript → topics\n"
        "4. Layer-1 dedup: inject existing topic list\n"
        "5. Generate 4-question bank per new topic\n"
        "6. Store nodes + contexts + edges in PKG\n"
        "Hook: rocky hook install --stop"
    )
    e, _ = box(40, 440, 540, 220, session, "#064e3b", "#10b981", fg="#d1fae5", size=12)
    el += e

    e, _ = note(620, 80, 420, 580,
                "📝 ANNOTATION — open questions:\n\n"
                "• Should session-end be idempotent for the same commit SHA?\n"
                "  (currently it processes each SHA once and deletes)\n\n"
                "• How to handle transcript > LLM context window?\n"
                "  Current: truncate to most recent tokens\n\n"
                "• Should rocky explore auto-refresh after N days?\n\n"
                "• Should layer-1 dedup use cosine similarity instead of\n"
                "  prompt injection? (more reliable but needs embeddings)\n\n"
                "• Should there be a rocky session-end --dry-run mode?")
    el += e

    save("07_rich_context_pipeline.excalidraw", el)


# ── 08: Rocky IQ & UI ─────────────────────────────────────────────────────────
def diagram_rocky_iq_ui():
    el = []
    e, _ = txt(40, 20, "08 · Rocky IQ & Web UI — Dashboard, Map, Queue, Sessions, Projects",
               color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 52, "src/server.rs · src/app.html", color="#475569", size=11)
    el += e

    iq = (
        "Rocky IQ  =  round((1 - atrophy_score) × 100)\n"
        "─────────────────────────────────────────────────────────\n"
        "For each node created in the last 60 days:\n"
        "  w     = max(0, 1 - days_since_created / 60)\n"
        "  R     = retrievability(stability, last_reviewed)\n"
        "  decay = max(0, 0.7 - R) / 0.7\n"
        "atrophy = Σ(w × decay) / Σ(w)\n\n"
        "≥ 80 → green (strong)  |  70–79 → yellow  |  < 70 → red"
    )
    e, _ = box(40, 80, 560, 180, iq, "#064e3b", "#10b981", fg="#d1fae5", size=12)
    el += e

    tabs = (
        "5-TAB SIDEBAR LAYOUT  (rocky view → localhost)\n"
        "─────────────────────────────────────────────────────────\n"
        "#dashboard  →  Rocky IQ banner · due-for-review · domain health bars\n"
        "               recently added nodes · projects mini-list\n\n"
        "#map        →  PixiJS WebGL force-directed graph\n"
        "               nodes coloured by R · planning mode toggle\n\n"
        "#queue      →  sortable table (recall/recent/reviews/alpha)\n"
        "               filter: all/due/critical · 'Quiz top 5' button\n\n"
        "#sessions   →  timeline grouped by created_at date\n"
        "               encounter_count ×N badges\n\n"
        "#projects   →  chord diagram · domain-mix donut · knowledge timeline"
    )
    e, _ = box(40, 280, 560, 280, tabs, "#0f172a", "#334155", fg="#cbd5e1", size=12)
    el += e

    e, _ = note(640, 80, 400, 480,
                "📝 ANNOTATION — open questions:\n\n"
                "• Should Rocky IQ be per-project or global?\n\n"
                "• Is 60-day recency window the right cutoff?\n"
                "  (older topics don't affect IQ score)\n\n"
                "• Should Dashboard show a review streak counter?\n\n"
                "• Should Review Queue support bulk-quiz\n"
                "  (select multiple topics, run as one session)?\n\n"
                "• Should the Knowledge Map have a search/filter bar?\n\n"
                "• Should Projects tab show per-project Rocky IQ?")
    el += e

    save("08_rocky_iq_and_ui.excalidraw", el)


# ── 09: Voice Architecture ────────────────────────────────────────────────────
def diagram_voice():
    el = []
    e, _ = txt(40, 20, "09 · Voice Architecture — Push-to-Talk + whisper.cpp Backend",
               color="#f59e0b", size=18)
    el += e
    e, _ = txt(40, 52, "src/voice.rs · src/server.rs /api/transcribe · scripts/install-whisper.sh",
               color="#475569", size=11)
    el += e

    tiers = (
        "THREE PROVIDER TIERS\n"
        "─────────────────────────────────────────────────────────\n"
        "Tier 1  provider = 'off'  (default)\n"
        "  Mic button hidden. No audio ever captured.\n\n"
        "Tier 2  provider = 'whisper-cpp'  (recommended)\n"
        "  Runs whisper-cli subprocess locally.\n"
        "  Model: ggml-base.en.bin (~142 MB in ~/.rocky/models/)\n"
        "  No network call at transcription time.\n"
        "  Works with privacy.strict = true.\n\n"
        "Tier 3  provider = 'browser'  (convenience)\n"
        "  Uses browser Web Speech API (sends audio to Google/OS).\n"
        "  Requires browser_consent = true in config.\n"
        "  Blocked when privacy.strict = true."
    )
    e, _ = box(40, 80, 540, 260, tiers, "#0f172a", "#475569", fg="#cbd5e1", size=12)
    el += e

    flow = (
        "PUSH-TO-TALK FLOW (web UI)\n"
        "─────────────────────────────────────────────────────────\n"
        "1. User holds 🎤 button (mousedown)\n"
        "2. getUserMedia → Web Audio ScriptProcessor captures PCM\n"
        "3. User releases button (mouseup)\n"
        "4. Browser: resampleLinear(pcm, srcRate, 16000)\n"
        "            encodeWAV(samples) → ArrayBuffer\n"
        "5. POST /api/transcribe  Content-Type: audio/wav\n"
        "6. Server: spawn_blocking → whisper-cli tempfile\n"
        "7. Server: read .txt output → { 'text': '...' }\n"
        "8. Browser: insert transcript into answer textarea"
    )
    e, _ = box(40, 360, 540, 240, flow, "#0c4a6e", "#06b6d4", fg="#e0f2fe", size=12)
    el += e

    e, _ = note(620, 80, 420, 520,
                "📝 ANNOTATION — open questions:\n\n"
                "• Should there be a rocky voice test command\n"
                "  to verify end-to-end setup?\n\n"
                "• Should whisper model be configurable?\n"
                "  tiny vs base vs small (speed vs accuracy)\n\n"
                "• Should browser TTS be wired up for reading\n"
                "  questions aloud? (tts = 'browser' config key)\n\n"
                "• Push-to-hold vs push-to-toggle — which is better\n"
                "  UX for long answers?\n\n"
                "• Should the mic button show a live waveform?\n\n"
                "• CLI hands-free mode (rocky quiz --voice):\n"
                "  needs cpal + webrtc-vad — deferred from v0.2")
    el += e

    save("09_voice_architecture.excalidraw", el)


# ── Main ──────────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    print("Generating Rocky study diagrams...\n")
    random.seed(42)  # reproducible IDs
    diagram_lifecycle()
    diagram_data_model()
    diagram_quiz_flow()
    diagram_discovery()
    diagram_fsrs()
    diagram_classification()
    diagram_rich_context_pipeline()
    diagram_rocky_iq_ui()
    diagram_voice()
    print(f"\nDone. Open any .excalidraw file in VS Code (Excalidraw extension)")
    print(f"or drag onto https://excalidraw.com")
