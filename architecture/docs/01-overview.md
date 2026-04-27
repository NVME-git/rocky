# Rocky — architecture overview

Rocky is a single Rust CLI binary backed by SQLite at `~/.rocky/graph.db`. It
captures topics from git diffs and Claude Code activity, then surfaces them
through spaced-repetition quizzes.

The container view shows the eleven internal containers; the component views
drill into the four most-touched ones — CLI, Teacher, PKG, FSRS.

For the why-behind-each-subsystem see the ADRs (Decisions tab).
