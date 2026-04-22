# Rocky — NotebookLM Source Document
## LinkedIn Alpha Tester Outreach

**How to use this file:**
1. Go to [notebooklm.google.com](https://notebooklm.google.com)
2. Create a new notebook — upload this file as the only source
3. Click **Audio Overview** → Generate
4. Download the MP3, add a simple screen recording or slide deck on top, export as MP4
5. Post to LinkedIn with the caption block at the bottom of this file

---

## The Problem: AI Is Making Senior Developers Forget How to Code

Something strange has been happening in engineering teams over the last two years.

Senior developers who have been writing code for a decade are losing their ability to spot bugs in their own pull requests. They're struggling to answer basic interview questions for their own stack. They can't explain, off the top of their head, why a piece of code they shipped six months ago works the way it does.

The culprit isn't burnout. It's not job-hopping. It's AI pair programmers.

When GitHub Copilot, Claude, or ChatGPT writes your code, you ship faster. That part is real. But every time the AI does the thinking for you — every time you accept a completion without fully understanding it — you are making a small withdrawal from your own knowledge bank. Individually, each withdrawal is invisible. Compounded across hundreds of coding sessions over two years, the result is what researchers are starting to call **AI skill atrophy**.

Here's how it shows up in practice:

- You can describe what a piece of code does, but you can't explain *why* the edge case on line 47 matters.
- You know the API call works. You have no idea what happens when it times out.
- You reviewed the PR. You approved it. You couldn't reproduce the bug it introduced three weeks later.

The hardest part about AI skill atrophy is that it's invisible until the moment it isn't. You feel productive every day. Your velocity metrics look fine. And then one day the AI gives you a subtly wrong answer, and you don't catch it — because you no longer have the depth to know it's wrong.

This is the problem Rocky was built to solve.

---

## What Rocky Is

Rocky is a command-line tool that runs silently alongside your AI coding workflow and fights skill atrophy at its source.

It works like this: every time you make a commit — especially a commit where an AI wrote some or all of the code — Rocky reads the diff. It figures out what you actually needed to understand to write that code correctly. And then it asks you about it. Not a quiz. Not a flashcard. A targeted technical question about the actual code you just shipped.

Not "What is JWT?" — but "You're using a 15-minute access token expiry here. What happens to users who are mid-session when that token expires, and how does your refresh token rotation handle it?"

That's the kind of question that separates someone who read the docs from someone who has actually built production systems with it. Rocky asks the second kind — every time, about your own code.

---

## How Rocky Works: The Technical Architecture

Rocky builds what it calls a **Personal Knowledge Graph** — a local SQLite database that lives on your machine and tracks every technical concept you've encountered in your codebase.

Each concept in the graph is a node. Nodes have a **retrievability score** — a number between 0 and 1 that represents how well you'd recall this concept right now if you were asked about it. This score decays over time using a formula borrowed from spaced-repetition research: the FSRS algorithm. The longer since you last demonstrated understanding of a concept, the lower its retrievability score drops.

When the score drops below 0.7, Rocky considers the concept "fading." Below that, it's a "gap." Rocky prioritises gaps and fading topics when it decides what to quiz you on next.

### The Three-Stage Context Pipeline

Rocky's newest feature is what the team calls the rich-context pipeline. Here's how it works:

**Stage 1 — Explore.** When you add Rocky to a project, you run `rocky explore`. Rocky reads your README, your git history, your dependency manifest — and builds a project context summary. This summary gets injected into every future question it generates, so the questions are grounded in your actual codebase, not generic examples.

**Stage 2 — Post-commit queue.** Every time you commit, a git hook fires. But instead of immediately calling an AI model — which would slow down your commit — it just queues the commit SHA. The hook takes under five milliseconds.

**Stage 3 — Session-end enrichment.** When you close your Claude Code session, a Stop hook fires. Rocky drains the commit queue, reads the git diffs, reads the last several hours of your Claude Code session transcript, and sends everything to the AI in one rich batch call. Because it has the conversation context — what you and the AI were discussing, what you were trying to build, what problems came up — the questions it generates are dramatically more specific and relevant.

### Rocky IQ

Rocky computes a score it calls **Rocky IQ**: a single number from 0 to 100 that represents what fraction of your recently-learned knowledge you're still retaining. It weights recent topics more heavily than old ones, using a 60-day sliding window. A score above 80 is strong. Below 70 means you have significant knowledge gaps opening up. Below 60 is a warning sign.

### Voice Mode

Rocky v0.2 ships with voice support. In the web UI — which runs locally on your machine — there's a push-to-talk microphone button next to the answer box. You hold it, speak your answer, and Rocky transcribes it locally using whisper.cpp. No audio leaves your machine. Then it evaluates whether your spoken answer demonstrates genuine understanding.

There's also a CLI voice mode: `rocky quiz --voice` reads questions aloud using your system's text-to-speech engine and records your spoken answer via your microphone.

### Privacy-First Design

Everything runs locally by default. Your git diffs, your Claude Code transcripts, your quiz answers — none of it leaves your machine unless you explicitly configure a cloud LLM provider. Rocky works with a local Ollama model for complete air-gap operation. There's a `privacy.strict` config flag that refuses to start if any cloud provider is configured.

---

## The Interface: What Using Rocky Actually Feels Like

You finish a coding session. You close Claude Code. Behind the scenes, Rocky has already fired its Stop hook, read your transcript, analysed your commits, and generated five targeted questions about what you just built.

The next morning you run `rocky quiz`. Rocky shows you the topics that have decayed the most since you last reviewed them. It asks you a question — specific, grounded, about your actual code. You type an answer. Rocky evaluates it: not looking for keywords, but for genuine comprehension. If you're close but missing something, it asks a follow-up. If you're clearly lost, it explains the concept from scratch and marks it for early resurfacing.

When you open `rocky view` in the browser, you see a force-directed knowledge graph of everything you've learned — nodes coloured by retrievability, edges showing conceptual relationships. There's a dashboard with your Rocky IQ score, a list of concepts due for review today, and a breakdown of which knowledge domains are fading fastest.

You can also run `rocky dedupe` when your graph has grown large. Rocky finds pairs of near-duplicate topics — like "exponential backoff" and "retry with exponential backoff" — shows them to you side by side, and lets you merge them. You can keep one name, keep the other, or ask the AI to pick the better canonical name from both.

---

## Why This Matters Beyond Individual Developers

The skill atrophy problem isn't just personal. It's becoming a hiring and team-reliability problem.

Engineering managers are starting to report that developers who've been working heavily with AI tools for two or more years are harder to evaluate in technical interviews — not because they've become less competent on paper, but because their on-the-spot recall has degraded. They can architect systems beautifully. They struggle to explain the fundamentals underneath without Googling.

Senior engineers are becoming more expensive to replace, because when they leave, they take not just their code knowledge but their ability to spot bugs in AI-generated code — a skill that is increasingly rare and increasingly valuable.

The teams that will win the next five years of software development aren't the ones that adopt AI the fastest. They're the ones that adopt AI intelligently — pairing it with intentional knowledge preservation so their engineers stay sharp even as they become more leveraged.

Rocky is designed to be that discipline layer.

---

## What Alpha Testers Would Be Testing

Rocky is in active development. The core features are working:

- The rich-context pipeline (explore → post-commit queue → session-end)
- The Personal Knowledge Graph with FSRS-based decay
- The web UI with Knowledge Map, Dashboard, Review Queue, Sessions, and Projects tabs
- Voice input in the web UI (whisper.cpp, runs locally)
- Rocky IQ score
- `rocky dedupe` for graph cleanup
- Ollama support for fully local operation
- The tutorial script for a one-command smoke test

What alpha testers would be helping with:

1. **Real-world question quality.** The most important signal. Are the questions Rocky generates actually relevant and useful? Are they too easy, too vague, too generic? This is the core value proposition — we need to know if it's working.

2. **Workflow friction.** Does the hook setup feel natural? Does the session-end timing work, or does it fire at bad moments? Is the CLI interface comfortable to use daily?

3. **Ollama model recommendations.** Rocky works with any Ollama model. We need to know which models produce the best questions on real codebases.

4. **PKG growth patterns.** What does a real PKG look like after 30 days of active use? Are the topics clean, or do they degrade into noise?

5. **Edge cases we haven't hit.** Monorepos, non-English codebases, very large diffs, projects with no README — we want to see how Rocky behaves in the wild.

**The ideal alpha tester:** A developer who is actively using Claude Code, GitHub Copilot, or a similar AI coding tool daily. Comfortable with CLIs. Happy to share honest, sometimes harsh feedback. The more opinionated, the better.

---

## The Vision

The long-term vision for Rocky is a tool that every serious developer keeps running in the background — the way they keep a linter running. Invisible most of the time. Only surfacing when something needs attention.

We want Rocky IQ to become a number developers track the way they track their health metrics — a leading indicator of whether their technical skills are growing, holding, or quietly eroding.

We want the Personal Knowledge Graph to become a developer's portable intellectual property — a structured record of everything they genuinely understand, that they own, that lives on their machine, and that they can carry with them across jobs, projects, and years.

We want the question quality to reach a point where a developer can say: "Rocky asked me something I hadn't thought about since I shipped it, and the answer changed how I build the next feature."

That's what we're building toward. And we need sharp, honest developers to help us get there.

---

## Call to Action

If you're a developer using AI coding tools daily and you're even a little worried about what it's doing to your skills — Rocky is worth trying.

If you think the problem of AI skill atrophy is real and worth solving — Rocky is worth contributing to.

The project is open source. The install is one cargo command. The tutorial takes ten minutes to walk through the full flow.

**GitHub:** https://github.com/NVME-git/rocky
**Docs:** https://nvme-git.github.io/rocky

Early feedback is the most valuable thing we can get right now. If Rocky generates a question that's too vague, we want to know. If it misses something obvious from your diff, we want to know. If the workflow is annoying, we want to know.

The only way to make Rocky genuinely useful is to run it against real codebases with real AI-assisted development workflows. If that describes your daily work, we'd love your help.

---

## LinkedIn Post Caption

Use this alongside the video when posting:

---

**I built a tool to fight the thing nobody's talking about: AI skill atrophy.**

Every time your AI writes your code, you ship faster. But you also practice less.

After two years of heavy AI-assisted development, a lot of senior engineers can't explain their own codebase off the top of their head. They've traded depth for velocity — often without noticing.

Rocky is a CLI that runs alongside your AI workflow and fights this at the source.

After every commit, it reads your diff, figures out what you needed to understand to write it correctly, and asks you about it — in the context of your actual code.

Not "what is JWT?" — but "you're using 15-minute access tokens here. What breaks for users who are mid-session when one expires?"

It builds a Personal Knowledge Graph of your skills, tracks decay using spaced repetition, and gives you a Rocky IQ score: how much of what you've recently learned are you still retaining?

It runs entirely locally. Your code never leaves your machine.

We're looking for alpha testers — specifically developers who use Claude Code or Copilot daily and are willing to give honest, critical feedback.

If that's you: link in bio. DM me. Happy to walk you through setup.

**#AItools #SoftwareEngineering #DeveloperTools #OpenSource #LearningAndDevelopment**

---

*Source document prepared for NotebookLM Audio Overview generation.*
*Upload this file as the sole source, then click Audio Overview → Generate.*
