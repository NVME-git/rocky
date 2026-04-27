workspace "Rocky" "Personal Knowledge Graph — spaced repetition for developers" {

    model {

        # ── People ────────────────────────────────────────────────────────────
        dev = person "Developer" "Uses Rocky to track and test understanding of code topics."

        # ── External systems ─────────────────────────────────────────────────
        anthropic   = softwareSystem "Anthropic API" "Claude LLM — generates questions, evaluates answers, classifies topics." "External"
        ollama      = softwareSystem "Ollama" "Local LLM runtime. Drop-in alternative to Claude when [privacy].strict or no API key." "External"
        git         = softwareSystem "Git / GitHub" "Source of commits, diffs, project history." "External"
        claudeCode  = softwareSystem "Claude Code" "AI coding assistant. Runs UserPromptSubmit + Stop hooks and loads /rocky-checkpoint and /rocky-quiz skills." "External"
        whispercpp  = softwareSystem "whisper.cpp" "Local STT subprocess for push-to-talk answer capture." "External"
        obsidian    = softwareSystem "Obsidian Vault" "Filesystem destination for the markdown export of the PKG (pkg/)." "External"
        pkgRemote   = softwareSystem "PKG git remote" "GitHub repo (or any git remote) holding pkg/pkg.json + per-domain markdown for cross-machine sync." "External"

        # ── Rocky ────────────────────────────────────────────────────────────
        rocky = softwareSystem "Rocky" "CLI tool that builds and queries a Personal Knowledge Graph from code activity." {

            !docs docs
            !adrs decisions

            cli = container "rocky CLI" "Entry point for all commands. Single Rust binary." "Rust / Clap / src/main.rs" {

                # Manual flows
                taskCmd       = component "run_task"         "User describes pre-work intent; classify → Socratic Q&A before coding."          "src/main.rs"
                diffCmd       = component "run_diff"         "Analyse a commit / staged diff. Manual path — calls Teacher directly."           "src/main.rs"
                quizCmd       = component "run_quiz"         "Priority queue: queued → most-overdue → recent prompts. Drives review loop."     "src/main.rs"
                inspectCmd    = component "run_inspect"      "Print full node detail: contexts, source commits, question bank."                "src/main.rs"
                lsCmd         = component "run_list"         "List all topics; --json mode used by the rocky-checkpoint skill for dedup."      "src/main.rs"
                dueCmd        = component "run_due"          "Topics due for review as JSON. Used by the rocky-quiz skill."                    "src/main.rs"
                topicCmd      = component "run_topic"        "Look up a single topic as JSON (question bank + history)."                       "src/main.rs"
                reviewCmd     = component "run_review"       "Record a quiz outcome. Updates FSRS state."                                       "src/main.rs"
                addTopicCmd   = component "run_add_topic"    "Skill-driven topic insert. Bypasses the in-Rocky LLM call."                      "src/main.rs"
                addQCmd       = component "run_add_question" "Append a question to a topic's question_bank."                                    "src/main.rs"
                deleteCmd     = component "run_delete"       "Search + delete topics from the PKG."                                             "src/main.rs"

                # Rich-context pipeline
                exploreCmd    = component "run_explore"      "Summarise CLAUDE.md / README / docs / recent commits → project_context table."   "src/main.rs"
                postCommitCmd = component "run_post_commit"  "Silent queue mode for the git post-commit hook. Appends to pending_diffs. No LLM." "src/main.rs"
                sessionEndCmd = component "run_session_end"  "Drains pending_diffs + reads transcript, generates rich nodes + question bank."  "src/main.rs"
                checkpointCmd = component "run_checkpoint"   "Read / drain the pending_diffs queue for the rocky-checkpoint skill (no LLM)."   "src/main.rs"
                contextCmd    = component "run_context"      "Emit the stored project_context summary as JSON for the skill."                  "src/main.rs"

                # Hooks + install
                hookCmd       = component "run_hook"         "Reads JSON from stdin; logs Claude Code prompt to ./.rocky (non-blocking)."      "src/main.rs"
                installCmd    = component "run_install"      "Install git / Claude Code hooks + Skills (writes ~/.claude/settings.json)."     "src/main.rs"

                # Maintenance
                viewCmd       = component "run_view"         "Boots the View Server."                                                          "src/main.rs"
                exportCmd     = component "run_export"       "Write all PKG topics to Markdown via the Obsidian exporter."                    "src/main.rs"
                syncCmd       = component "run_sync"         "Commit PKG, optionally init or push the vault repo."                             "src/main.rs"
                restoreCmd    = component "run_restore"      "Rebuild graph.db from pkg/pkg.json on a new machine."                            "src/main.rs"
                classifyCmd   = component "run_classify"     "Assign taxonomy domains to undomained topics via LLM."                          "src/main.rs"
                edgesCmd      = component "run_edges"        "List / summarise the implication graph."                                         "src/main.rs"
                dedupeCmd     = component "run_dedupe"       "Find and merge near-duplicate topics across projects."                          "src/main.rs"
                backfillCmd   = component "run_backfill"     "Walk git history → topics. --fill-clues / --fill-question-bank generate missing fields." "src/main.rs"
            }

            teacher = container "Teacher" "Wraps the configured LLM provider. Generates questions, evaluates answers, classifies topics." "Rust / src/teacher.rs" {
                providerSwitch = component "provider switch"                  "Routes calls to Claude or Ollama based on [llm].provider. Honours privacy.strict." "src/teacher.rs"
                genQ           = component "generate_question"                "Generates a question at Normal / Simpler / Harder difficulty."                    "src/teacher.rs"
                genQA          = component "generate_question_and_answer"     "Canonical Q + A + clue at backfill / session-end."                                "src/teacher.rs"
                genBank        = component "generate_question_bank"           "Bank of 4 implication-grounded Q+A+clue triples per topic."                       "src/teacher.rs"
                evalA          = component "evaluate_answer"                  "Scores a user answer 0.0–1.0, returns feedback + follow-up."                       "src/teacher.rs"
                genExpl        = component "generate_explanation"             "Full explanation when user types ? or exhausts attempts."                          "src/teacher.rs"
                genClue        = component "generate_clue"                    "Short Socratic hint for [c] option during quiz."                                   "src/teacher.rs"
                classT         = component "classify_topics"                  "Extracts topic list from a diff or prompt text."                                   "src/teacher.rs"
                crossQ         = component "generate_cross_concept_question"  "Question spanning two connected nodes via an edge."                                "src/teacher.rs"
                exploreSummary = component "summarise_project"                "Builds the project_context summary at `rocky explore` time."                       "src/teacher.rs"
            }

            pkg = container "PKG" "Personal Knowledge Graph. Single global SQLite at ~/.rocky/graph.db (or $ROCKY_HOME)." "SQLite / rusqlite / src/db.rs" {
                nodes        = component "nodes table"           "Topic nodes: stability, difficulty, retrievability inputs, canonical Q&A, clue, encounter_count, repos[]." "src/db.rs"
                edges        = component "edges table"           "Typed edges: implies, depends_on, conflicts_with, part_of. Strength + last_fired."                          "src/db.rs"
                contexts     = component "contexts table"        "Commit messages / task descriptions attached to nodes."                                                     "src/db.rs"
                reviews      = component "reviews table"         "Per-quiz record: question, answer, score, feedback."                                                         "src/db.rs"
                session      = component "session_state table"   "Daily budget, cooldown, sessions-since-push counter."                                                        "src/db.rs"
                projectCtx   = component "project_context table" "Per-project summary written by `rocky explore`. Grounds question generation."                                "src/db.rs"
                pendingDiffs = component "pending_diffs table"   "Queue of (sha, msg, diff) waiting to be turned into rich nodes by session-end."                              "src/db.rs"
            }

            fsrs = container "FSRS Engine" "Free Spaced Repetition Scheduler. Computes retrievability and updates stability." "Rust / src/fsrs.rs" {
                retrieve = component "retrieve()" "R = (1 + t / (9 × S)) ^ -1"                                                "src/fsrs.rs"
                classify = component "classify()" "R ≥ 0.9 → KNOWN | 0.7–0.9 → FADING | < 0.7 → GAP"                          "src/fsrs.rs"
                fsrsUpd  = component "update()"   "Update stability + difficulty after a review using the FSRS-4.5 update rule." "src/fsrs.rs"
            }

            localLog    = container ".rocky log"          "Per-project log file. Stores Claude Code prompt text for `rocky quiz` to consume. Auto-purged after 24h." "Plain file / src/local_log.rs"
            transcript  = container "Transcript Reader"   "Reads ~/.claude/projects/<slug>/*.jsonl and privacy-strips down to file paths + command names."         "Rust / src/transcript.rs"
            obsidianExp = container "Obsidian Exporter"   "Writes one .md per topic plus auto-generated dashboards into pkg_dir."                                  "Rust / src/obsidian.rs"
            syncMod     = container "Sync"                "Git operations on the PKG vault (init / commit / push / restore)."                                       "Rust / src/sync.rs"
            voice       = container "Voice"               "Push-to-talk wrapper. Spawns whisper-cli subprocess. Honours privacy.strict."                            "Rust / src/voice.rs"

            viewServer  = container "View Server"         "Axum HTTP server: serves app.html / view.html / view_projects.html and a JSON API at /api/data."        "Rust / Axum / src/server.rs"

            vsCodeExt   = container "VS Code Extension"   "Sidebar tree views of the PKG (Due / Topics / Sessions / Summary). Consumes /api/data."                  "TypeScript / extensions/vscode"
            browserExt  = container "Browser Extension"   "Capture YouTube / articles into the PKG while browsing."                                                 "TypeScript / extensions/browser"

            skills      = container "Claude Code Skills"  "rocky-checkpoint and rocky-quiz skills installed into ~/.claude/skills/. Drive extraction without an in-Rocky LLM call." "Markdown SKILL.md / skills/"
        }

        # ── People → Rocky ───────────────────────────────────────────────────
        dev -> cli        "rocky <command>"               "shell"
        dev -> vsCodeExt  "browse PKG inside the editor"  "VS Code UI"
        dev -> browserExt "capture web content"           "browser UI"

        # ── CLI components → Teacher components (in-process) ────────────────
        taskCmd       -> classT     "classify task description into topics" "Rust fn"
        taskCmd       -> genQ       "generate question for new topic"        "Rust fn"
        taskCmd       -> evalA      "evaluate user answer"                   "Rust fn"
        diffCmd       -> classT     "classify topics from diff"              "Rust fn"
        diffCmd       -> genQA      "generate canonical Q&A + clue"          "Rust fn"
        diffCmd       -> genClue    "generate clue when missing"             "Rust fn"
        diffCmd       -> genBank    "generate question bank"                 "Rust fn"
        quizCmd       -> genQ       "generate question for due topic"        "Rust fn"
        quizCmd       -> evalA      "evaluate user answer"                   "Rust fn"
        quizCmd       -> genClue    "generate hint when [c] pressed"         "Rust fn"
        quizCmd       -> genExpl    "generate full explanation when [?] pressed" "Rust fn"
        quizCmd       -> crossQ     "generate cross-concept question"         "Rust fn"
        backfillCmd   -> classT     "classify topics from historical diffs"  "Rust fn"
        backfillCmd   -> genQA      "fill canonical Q&A retroactively"       "Rust fn"
        backfillCmd   -> genClue    "fill clues retroactively"               "Rust fn"
        backfillCmd   -> genBank    "fill question bank retroactively"       "Rust fn"
        sessionEndCmd -> classT     "classify topics from queued commits"    "Rust fn"
        sessionEndCmd -> genQA      "generate Q&A per topic"                 "Rust fn"
        sessionEndCmd -> genBank    "generate 4-question bank per topic"     "Rust fn"
        sessionEndCmd -> genClue    "generate clue per topic"                "Rust fn"
        exploreCmd    -> exploreSummary "build project_context summary"      "Rust fn"
        classifyCmd   -> classT     "assign taxonomy domain"                 "Rust fn"

        # ── CLI components → FSRS components (in-process) ───────────────────
        quizCmd   -> retrieve "compute retrievability for due topics" "Rust fn"
        quizCmd   -> classify "classify into KNOWN / FADING / GAP"    "Rust fn"
        reviewCmd -> fsrsUpd  "update stability + difficulty"          "Rust fn"

        # ── CLI components → PKG components (rusqlite) ──────────────────────
        diffCmd       -> nodes        "insert/update topic nodes"          "rusqlite"
        diffCmd       -> edges        "insert implication edges"           "rusqlite"
        diffCmd       -> contexts     "attach commit message"              "rusqlite"
        backfillCmd   -> nodes        "insert nodes with commit-date timestamps" "rusqlite"
        backfillCmd   -> contexts     "attach commit messages"             "rusqlite"
        quizCmd       -> nodes        "read most-overdue topics"           "rusqlite"
        quizCmd       -> reviews      "write quiz review row"               "rusqlite"
        quizCmd       -> session      "increment daily counter"             "rusqlite"
        taskCmd       -> nodes        "insert nodes after Q&A"             "rusqlite"
        taskCmd       -> contexts     "attach task description"            "rusqlite"
        addTopicCmd   -> nodes        "upsert from skill-driven extraction" "rusqlite"
        addQCmd       -> nodes        "append to question_bank"             "rusqlite"
        deleteCmd     -> nodes        "remove nodes"                        "rusqlite"
        dedupeCmd     -> nodes        "merge duplicate nodes"               "rusqlite"
        edgesCmd      -> edges        "list / summarise edges"              "rusqlite"
        lsCmd         -> nodes        "read all topics"                     "rusqlite"
        dueCmd        -> nodes        "read overdue topics"                  "rusqlite"
        topicCmd      -> nodes        "read single topic"                    "rusqlite"
        reviewCmd     -> nodes        "bump encounter_count"                 "rusqlite"
        reviewCmd     -> reviews      "write review row"                     "rusqlite"
        inspectCmd    -> nodes        "read node + question bank"            "rusqlite"
        contextCmd    -> projectCtx   "read project context row"             "rusqlite"
        exploreCmd    -> projectCtx   "write project context row"            "rusqlite"
        postCommitCmd -> pendingDiffs "append (sha, msg, diff)"              "rusqlite"
        checkpointCmd -> pendingDiffs "drain pending_diffs"                  "rusqlite"
        sessionEndCmd -> pendingDiffs "drain pending_diffs"                  "rusqlite"
        sessionEndCmd -> nodes        "upsert nodes from session"            "rusqlite"
        restoreCmd    -> nodes        "rebuild from pkg.json"                "rusqlite"
        classifyCmd   -> nodes        "update node domain"                   "rusqlite"

        # ── CLI components → other internal containers ───────────────────────
        hookCmd       -> localLog    "write prompt + timestamp"            "filesystem"
        quizCmd       -> localLog    "read last 24h prompts"               "filesystem"
        sessionEndCmd -> transcript  "read last N hours of activity"        "filesystem"
        exportCmd     -> obsidianExp "write markdown + dashboards"         "Rust fn"
        diffCmd       -> obsidianExp "auto-export updated nodes"           "Rust fn"
        sessionEndCmd -> obsidianExp "auto-export new / updated nodes"     "Rust fn"
        syncCmd       -> syncMod     "init / commit / push the vault repo" "Rust fn"
        sessionEndCmd -> syncMod     "auto-commit when [sync].enabled"     "Rust fn"
        diffCmd       -> syncMod     "auto-commit when [sync].enabled"     "Rust fn"
        quizCmd       -> voice       "speak question, transcribe answer (--voice)" "Rust fn"
        viewCmd       -> viewServer  "boot the Axum server"                 "Rust fn"
        installCmd    -> skills      "copy SKILL.md into ~/.claude/skills/" "filesystem"

        # ── View Server → PKG components (rusqlite) ─────────────────────────
        viewServer -> nodes   "fetch all nodes for /api/data"   "rusqlite"
        viewServer -> edges   "fetch all edges for /api/data"   "rusqlite"
        viewServer -> reviews "fetch session activity for /api/data" "rusqlite"

        # ── Skills → CLI components (shell) ─────────────────────────────────
        skills -> checkpointCmd "rocky checkpoint diff / mark"  "shell exec"
        skills -> contextCmd    "rocky context"                  "shell exec"
        skills -> lsCmd         "rocky list --json"              "shell exec"
        skills -> addTopicCmd   "rocky add-topic ..."             "shell exec"
        skills -> dueCmd        "rocky due"                       "shell exec"
        skills -> topicCmd      "rocky topic <name>"              "shell exec"
        skills -> reviewCmd     "rocky review --score ..."        "shell exec"
        skills -> addQCmd       "rocky add-question"              "shell exec"

        # ── External edges ──────────────────────────────────────────────────
        providerSwitch -> anthropic  "POST /v1/messages"                          "HTTPS / JSON"
        providerSwitch -> ollama     "POST /api/generate"                         "HTTP / JSON"
        diffCmd        -> git        "git diff / git log"                          "git CLI"
        backfillCmd    -> git        "git log --all, git show"                     "git CLI"
        postCommitCmd  -> git        "git rev-parse / git show on the new commit" "git CLI"
        syncMod        -> pkgRemote  "git push / git pull"                         "git CLI"
        obsidianExp    -> obsidian   "write *.md + pkg.json"                       "filesystem"
        voice          -> whispercpp "spawn whisper-cli with audio buffer"         "subprocess"

        # ── Claude Code → Rocky ─────────────────────────────────────────────
        claudeCode -> hookCmd       "UserPromptSubmit hook → rocky hook"           "stdin / JSON"
        claudeCode -> sessionEndCmd "Stop hook → rocky session-end"                "subprocess"
        claudeCode -> skills        "loads SKILL.md when user types /rocky-checkpoint or /rocky-quiz" "Skill loader"

        # ── Extensions → View Server ────────────────────────────────────────
        vsCodeExt  -> viewServer "GET /api/data"     "HTTP / JSON"
        browserExt -> viewServer "POST /api/capture" "HTTP / JSON"
    }

    views {

        systemContext rocky "SystemContext" {
            include *
            autoLayout lr
            description "Rocky in context — developer, git, two LLM providers, Claude Code, whisper.cpp, Obsidian, the PKG vault remote."
        }

        container rocky "Containers" {
            include *
            autoLayout tb
            description "Internal containers: CLI, Teacher, PKG, FSRS, transcript reader, Obsidian exporter, sync, voice, view server, plus extensions and skills."
        }

        component cli "CLI_Components" {
            include *
            autoLayout tb
            description "CLI command handlers grouped by manual flow / rich-context pipeline / hooks / maintenance."
        }

        component teacher "Teacher_Components" {
            include *
            autoLayout tb
            description "Teacher functions plus the provider switch routing to Claude or Ollama."
        }

        component pkg "PKG_Components" {
            include *
            autoLayout tb
            description "SQLite tables that make up the PKG, including the rich-context queue (pending_diffs) and project_context."
        }

        component fsrs "FSRS_Components" {
            include *
            autoLayout tb
            description "FSRS spaced repetition functions."
        }

        styles {
            element "Person" {
                shape Person
                background #1e3a5f
                color #ffffff
            }
            element "Software System" {
                background #1e3a5f
                color #ffffff
            }
            element "External" {
                background #444444
                color #cccccc
            }
            element "Container" {
                background #0f4c81
                color #ffffff
            }
            element "Component" {
                background #1a6b9a
                color #ffffff
            }
            element "Database" {
                shape Cylinder
                background #0f4c81
                color #ffffff
            }
        }

        themes default
    }

    configuration {
        scope softwareSystem
    }
}
