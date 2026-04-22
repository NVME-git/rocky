# 0005 — Rocky IQ score + sidebar UI redesign

**Status:** Accepted (2026-04-22, alpha)

## Context

Two intertwined surfaces needed work:

1. **The web view had two tabs (Projects, Graph)** and treated the graph as the primary view. A graph is the wrong primary surface for *discovery* (what should I do next?) and *planning* (what's at risk?) — you can't scan it, you can't see decay at a glance, and clicking around is high-friction. The morning-check question — *"what's my state?"* — had no good answer in the existing UI.

2. **There was no headline metric.** PKG stats showed `7 known | 2 fading | 335 total` — an absolute count with no trend, no context, no aspiration. Users had nothing to root for.

The earlier internal discussion of a "morning-check" UI proposed an **AI Atrophy Score** (`████░░ 63% of recent topics decaying`) — but:
- *Atrophy* is a downer; users are conditioned to push numbers UP, not DOWN.
- The Atrophy framing makes higher = worse, which inverts the natural mental model of progress bars.

## Decision

**Rocky IQ = `(1 − atrophy) × 100`.** Inverts the metric so the user roots for it to *go up*. The underlying formula is unchanged from the atrophy proposal:

```
For each non-domain node:
  weight = max(0, 1 - days_since_created / 60)        # recent counts more
  decay  = max(0, 0.7 - retrievability)               # only nodes < 70% recall

atrophy_score = sum(weight × decay) / sum(weight) / 0.7   # → 0..1
Rocky IQ      = round((1 - atrophy_score) × 100)          # → 0..100
```

The 60-day half-window means topics older than 60 days don't drag the score around — Rocky IQ is a "*what about your recent work?*" measure, not a lifetime metric.

**Sidebar with five tabs.** Replace the topbar tab strip with a left sidebar, navigation by `data-tab`. Tabs and roles:

| Tab | Job |
|---|---|
| **Dashboard** | The morning check. Rocky IQ banner + Due-for-Review + Domain Health + Recently-Added + Projects mini-list. |
| **Knowledge Map** | The PixiJS graph. Filters (domain, repo), search, spread, and a **Planning mode** toggle that dims known nodes and highlights unlearned-adjacent topics — turns the map into a "what's next" view. |
| **Review Queue** | Sortable, filterable table. The CLI's `rocky ls` ported to a clickable interface. |
| **Sessions** | Timeline grouped by `created_at` date. `×N` badges where layer-1 dedup hit. |
| **Projects** | Per-repo health cards + cross-project chord + domain-mix donut + full-width knowledge timeline (toggle: by project / by domain). |

URL fragment routing (`#dashboard`, `#map`, …) for deep-links and screenshot tooling.

## Consequences

**Positive:**
- The user lands on a screen that answers *"what's my state and what should I do?"* in five seconds. The graph stays available for the times when topology actually matters.
- Rocky IQ = 71% reads as *"I'm holding it together, push to 80"*. Atrophy = 29% read as *"things are slipping, that's bad"*. Same data, vastly different motivational frame.
- Planning mode collapses two questions ("what do I know?" and "what's adjacent?") into one toggle.

**Negative / costs:**
- The map's PixiJS auto-ticker did not produce a visible first frame in some browser contexts after the layout changes (graph appeared blank until the user moved the mouse). Fixed by pre-converging the simulation 200 ticks and calling `app.renderer.render(app.stage)` synchronously after init — see commit `afae234`.
- Sessions and Queue tabs needed new backend data: `/api/sessions` endpoint, plus `atrophyScore`, `domainHealth`, `dueForReview`, `recentlyAdded` fields on `/api/data`. Modest, but real new surface area.
- The Projects tab's chord+timeline grew busy; we ended up replacing the second chord with a domain-mix donut and dropping the timeline below as a full-width band.

**Open question:**
- Voice will eventually want a sixth route (settings? voice config?). Defer until we know what voice needs.
