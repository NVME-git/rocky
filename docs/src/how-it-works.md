# How Rocky Works

Rocky has three modes of operation, each with different rules about when it quizzes you and what limits apply.

---

## Scenario 1: Manual task (`rocky "your task"`)

You describe what you're about to work on. Rocky extracts the topics, checks your PKG, and runs Socratic Q&A on anything new. No cooldown, no daily cap — you asked for it.

```mermaid
flowchart TD
    A([rocky &quot;build a caching layer&quot;]) --> B[Extract 2–5 key topics from task description]
    B --> C{For each topic}
    C --> D[Classify against PKG]
    D --> E{known?}
    E -- yes --> F[✓ Mark encountered, move on]
    E -- stale --> G[Write 2–3 sentence reminder\nUpdate PKG with small score bump]
    E -- new --> H[Generate Socratic question\nabout implications and trade-offs]
    H --> I{User answers}
    I -- understood --> J[Record in PKG with high confidence]
    I -- not understood --> K[Give senior-engineer explanation\nAsk one follow-up]
    K --> L[Record in PKG with partial confidence]
    I -- too easy --> M[Record as known — no Q&A needed]
    I -- skip --> N[Queue topic in .rocky\nNot added to PKG yet]
    I -- ignore --> P[Dismiss topic entirely\nnot added to PKG]
    F & G & J & L & M & N & P --> O([Done — summary printed])
```

---

## Scenario 2: Git commit hook (`rocky diff`)

After every `git commit`, Rocky analyses the diff for topics that appeared in your code. Cooldown and daily budget are enforced — this is automatic, not user-initiated. Topics that can't be quizzed right now are queued in `./.rocky` for the next `rocky quiz`.

```mermaid
flowchart TD
    A([git commit]) --> B[post-commit hook fires\nruns: rocky diff]
    B --> C[Read staged diff + commit message]
    C --> D[Extract topics from diff\nusing code-aware analysis]
    D --> E{Session checks}
    E -- cooldown active --> F[Queue all topics in ./.rocky\nPrint: Rocky ready in ~Xm]
    E -- daily budget reached --> G[Queue all topics in ./.rocky\nPrint: budget reached — resets tomorrow]
    E -- OK --> H{For each topic}
    H --> I[Classify against PKG]
    I --> J{known?}
    J -- yes --> K[Mark encountered, silent]
    J -- stale --> L[Print reminder to terminal]
    J -- new --> M[Run Socratic Q&A in terminal]
    M --> N{understood?}
    N -- yes --> O[Record in PKG]
    N -- no --> P[Explain answer\nRecord partial confidence]
    N -- skip --> Q[Queue in ./.rocky]
    K & L & O & P & Q --> R([Done])
```

---

## Scenario 3: Claude Code hook (`rocky hook`)

When Claude Code is used in a project with Rocky configured, each prompt is silently logged to `./.rocky`. No quiz happens here — this is just capture.

```mermaid
flowchart TD
    A([Claude Code prompt submitted]) --> B{Is rocky hook installed\nin .git/hooks/post-commit?}
    B -- no --> C([Nothing happens])
    B -- yes --> D[Log prompt text to ./.rocky SQLite\nAuto-delete entries older than 24h]
    D --> E([Silent — no output to user])
```

---

## Scenario 4: `rocky quiz`

Explicitly request a learning session. No limits apply. Rocky works through a priority queue: queued topics first, then PKG topics most overdue for review, then anything new from recent prompts in `./.rocky`.

```mermaid
flowchart TD
    A([rocky quiz]) --> B[Load queued topics from ./.rocky\nordered by queue time]
    B --> C{Queued topics exist?}
    C -- yes --> D[Quiz each queued topic\nRemove from queue after completion]
    C -- no --> E
    D --> E[Load PKG topics sorted by urgency\nlowest retrievability first]
    E --> F{Stale or gap nodes in PKG?}
    F -- yes --> G[Quiz each — stale gets reminder first\ngap gets full Socratic Q&A]
    F -- no --> H
    G --> H{Rocky hook installed\nin this project?}
    H -- yes --> I[Read prompts from ./.rocky\nfrom the last 24 hours]
    I --> J[Extract new topics from prompt history]
    J --> K{New topics found?}
    K -- yes --> L[Run Socratic Q&A on new topics]
    K -- no --> M
    H -- no --> M
    L --> M([Summary printed])
```

---

## Scenario 5: Pre-commit review (`rocky diff --staged`)

Review your staged changes before committing. Behaves like the manual flow — no limits.

```mermaid
flowchart TD
    A([rocky diff --staged]) --> B[Read git diff --staged output]
    B --> C[Extract topics from staged changes]
    C --> D{For each topic}
    D --> E[Classify against PKG]
    E --> F{known?}
    F -- yes --> G[✓ Mark encountered]
    F -- stale --> H[Print reminder]
    F -- new --> I[Run Socratic Q&A]
    I --> J{understood?}
    J -- yes --> K[Record in PKG]
    J -- no --> L[Explain + follow-up\nRecord partial confidence]
    J -- skip --> M[Queue in ./.rocky]
    G & H & K & L & M --> N([Proceed with commit])
```

---

## How the PKG classifies topics

Every topic in the PKG has a **retrievability score** — an estimate of how likely you are to recall it right now, based on how long ago you last reviewed it and how stable your knowledge is.

```mermaid
flowchart LR
    A[Topic reviewed] --> B[Stability score increases\nbased on answer quality]
    B --> C[Retrievability decays over time\nfaster for unstable topics]
    C --> D{Retrievability}
    D -- ≥ 90% --> E[known\nskipped automatically]
    D -- 70–90% --> F[stale\nprinted as reminder]
    D -- < 70% --> G[gap\nfull Socratic Q&A]
```

Higher stability means the topic decays slower — if you've demonstrated solid understanding multiple times, Rocky won't ask you about it again for weeks.

---

## Domain taxonomy

Every topic is assigned to one of 13 domains when it's first extracted. Domains group topics in the vault into subfolders and are used for Obsidian graph view clustering.

| Domain | Examples |
|---|---|
| Language | Rust lifetimes, Python decorators, Go channels |
| Database | SQL indexes, Redis TTL, Postgres transactions |
| Auth | JWT, OAuth2, RBAC, session tokens |
| API | REST design, GraphQL, WebSockets |
| Frontend | React hooks, DOM events, CSS layout |
| DevOps | Docker networking, CI/CD pipelines |
| Architecture | Event sourcing, retry patterns, microservices |
| Performance | Caching strategies, query optimisation |
| Security | OWASP, encryption, input validation |
| Testing | Unit vs integration, mocking, TDD |
| Tooling | Build systems, package managers |
| Data | Algorithms, data structures, ML concepts |
| Other | Anything that doesn't fit above |

Use `rocky classify` to assign domains to any older topics that predate this feature.
