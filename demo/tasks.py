"""
Demo task sequence — a developer building a web app over several sessions,
delegating each task to an AI agent and running PKG before each one.

Each task is designed to exercise a different graph state:
  Task 1  — hits a known topic (REST API), a stale topic (SQL optimization),
             and introduces a new one (connection pooling)
  Task 2  — introduces Redis and cache invalidation as new topics
  Task 3  — Alembic/migrations likely new; may overlap with SQL knowledge
  Task 4  — SQL optimization is now either refreshed (stale→known) or re-quizzed
  Task 5  — JWT is almost certainly new; tests the full Socratic loop
"""

TASKS = [
    (
        "add PostgreSQL with connection pooling to my Flask API",
        "Overlaps REST API (known) + SQL optimization (stale) + introduces connection pooling (new)",
    ),
    (
        "implement Redis caching for expensive database queries",
        "HTTP caching may surface as stale; Redis specifics are new",
    ),
    (
        "add database schema migrations using Alembic",
        "New topic; tests whether SQL knowledge transfers to migration concepts",
    ),
    (
        "optimize slow product listing queries — add indexes and review query plans",
        "SQL optimization was stale in task 1; by now it may be refreshed or re-quizzed",
    ),
    (
        "add JWT authentication with role-based access control to the API",
        "JWT and RBAC are almost certainly new — full Socratic loop expected",
    ),
]
