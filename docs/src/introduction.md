# What is Rocky?

Rocky is a command-line tool that keeps your coding skills sharp while AI agents handle more and more of your work.

## The problem it solves

When you use AI tools like Claude Code, GitHub Copilot, or ChatGPT to write code for you, you get things done faster — but you stop practising the skills yourself. Over time, you might find you can no longer write certain things without the AI's help. This is called **AI skill atrophy**.

Rocky fights this by quizzing you on the topics your AI agent is handling, so you stay genuinely sharp even as you delegate more work.

## How it works

1. Before starting a task, you tell Rocky what you're about to work on
2. Rocky figures out what technical topics are involved
3. For topics you already know well — Rocky stays quiet
4. For topics you haven't touched in a while — Rocky gives you a quick reminder
5. For brand new topics — Rocky asks you a question to make sure you actually understand it

Rocky tracks everything in a **Personal Knowledge Graph (PKG)** — a local database of topics you know, how well you know them, and how recently you've used them. Knowledge fades over time if you don't use it, so Rocky brings things back up when they need refreshing.

## What makes it different

Rocky doesn't ask "what is a database index?" — that's just a vocabulary quiz. Instead it asks things like "you're adding an index to a table that gets 2 million inserts per night — what happens to that bulk job?" That's the kind of question that reveals whether you actually understand something.

## Key concepts

| Term | What it means |
|---|---|
| **PKG** | Personal Knowledge Graph — your local database of topics |
| **Known** | You reviewed this recently and your recall is strong (90%+) |
| **Fading** | You know it but haven't used it in a while (70–90% recall) |
| **Gap** | You haven't learned this yet, or your recall has dropped too low |
| **Retrievability** | Rocky's estimate of how likely you are to remember something right now |
