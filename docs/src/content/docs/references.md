---
title: "References & Research"
order: 11
---
Rocky's design is informed by research and practitioner perspectives on learning, memory, and the impact of AI on technical skill. This section collects sources that shaped features or prompted new thinking.

---

## AI, Competence, and the Role of Struggle in Learning

**FreeCodeCamp Podcast — Dr. Mark Mahoney** · April 2026

*Watch the full interview: [youtu.be/Tb6oaEkxtp8](https://youtu.be/Tb6oaEkxtp8?si=W_vu3K-0WknmvOTI)*

Dr. Mark Mahoney is a computer science professor at Carthage College with over 20 years of teaching experience and the creator of [Playback Press](https://playbackpress.com), a platform for interactive programming education. This interview with Quincy Larson on the FreeCodeCamp podcast covers the impact of AI on programming education and professional practice.

---

### The competence vs. confidence gap

Mahoney's sharpest observation: AI tools give learners *confidence* without necessarily giving them *competence*. You can complete a project, see it work, feel good about it — and still be unable to reason through it independently the next time.

This is a new form of tutorial hell. The old version meant watching someone else solve problems without ever solving them yourself. The AI version means having problems solved *for* you, in code that runs, ships, and disappears into your codebase — leaving no trace of the struggle that would have built real understanding.

Rocky's response to this is the core of its design: every topic must pass a Socratic question grounded in consequences, not definitions. But Mahoney's point pushes further — the source of a topic's entry into the PKG matters as much as whether you can answer a question about it. A topic you encountered in your own debugging is different from a topic you saw in AI-generated code. **[AI-source tagging](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** is the planned feature that makes this distinction explicit.

---

### "The hard way" builds resilience

Mahoney expresses concern that students who rely on AI miss the grind of debugging — the hours spent staring at a problem with no help coming. That struggle isn't just inefficiency; it builds the resilience and pattern recognition that makes a developer effective under pressure.

This maps directly to Rocky's scaffolding options — clues, explanations, simplification. These are useful when genuinely stuck, but used reflexively they become an escape from the productive discomfort that creates competence. **[Hard mode](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** is the planned response: a configuration that removes the escape routes, forcing genuine engagement or an honest skip.

---

### Debugging as the durable skill

When asked which skills AI won't replace, Mahoney's answer is immediate: problem-solving and debugging. Not because AI can't debug — it can — but because the *judgment* to know when an AI's debug is wrong requires the same forensic instincts that only come from having debugged things yourself.

Rocky's current question format asks about implications and consequences. **[Debugging-focused questions](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** extend this into forensic territory: given this code change, what would a production failure look like? What would the stack trace tell you? These questions can't be answered by pattern-matching on documentation — they require the kind of thinking Mahoney identifies as durable.

---

### Iterative planning before code

Mahoney describes his own AI workflow: use the tool to iterate on a *plan* first, refuse to let it generate code until the plan is solid. This disciplines the collaboration — the developer stays in the decision seat, and the AI handles execution within defined constraints.

Rocky's pre-task mode (`rocky "task description"`) already reviews what you know before work begins. **[Pre-task gap framing](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)** makes the AI-risk dimension explicit: for each gap topic, Rocky tells you *this is something AI will likely write for you — here is the question you should be able to answer before trusting the output*. The pre-task session becomes a readiness check for supervised AI use, not just a general review.

---

### Motivation as the irreplaceable human element

Mahoney's view of his primary role as a professor: not to deliver information — AI can do that — but to motivate, inspire passion, and model what it looks like to care deeply about the craft. An LLM can explain recursion; it cannot make a student feel that recursion is worth understanding.

Rocky takes a different angle on this: Rocky the alien is enthusiastic, direct, and genuinely invested in your progress. The personality isn't decoration — it's an attempt to make the quiz feel like a conversation with someone rooting for you, not a test you're taking alone. That's a limited version of what Mahoney describes, but it's the right direction.

---

### Rocky as a required PR check

One idea that emerged from this discussion: if AI is handling more and more of the code in a pull request, what guarantees does a reviewer have that the author understands what they're merging?

The conventional answer is code review. But code review is good at catching logic errors, not at detecting whether the author could reason through the code without the AI that wrote it.

Rocky's PR check (**[planned feature](https://github.com/NVME-git/rocky/blob/main/BACKLOG.md)**) addresses this at the workflow level: before a PR can merge, Rocky verifies the author's PKG shows adequate recall on the topics introduced. Paired with GitHub's stacked PRs feature — where PRs build on each other in a reviewable stack — this creates a layer-by-layer knowledge check: each PR in the stack must demonstrate understanding of only what it introduces.

This doesn't slow down shipping. It makes the assumption behind shipping — *"the author knows what this does"* — verifiable.

---

*Have a paper, talk, or post that shaped your thinking on learning and AI? Open an issue or PR.*
