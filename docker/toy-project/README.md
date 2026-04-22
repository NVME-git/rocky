# urlshort

A tiny URL shortener used as a demo target for Rocky.

## Stack

- Node.js 20 + TypeScript
- Express for HTTP
- SQLite (better-sqlite3) for persistence
- Base62 short IDs
- Sliding-window rate limiting on `POST /shorten`
- URL validation that blocks `javascript:` and `data:` schemes

## Endpoints

| Method | Path        | Purpose                                  |
|--------|-------------|------------------------------------------|
| POST   | `/shorten`  | Body `{ url }` → `{ short }`             |
| GET    | `/:id`      | 302 redirect to the original URL          |
| GET    | `/health`   | Liveness probe                            |

## Why this project

It's small (~200 LOC) but covers enough real concepts for Rocky to ask
genuinely useful questions: HTTP routing, async storage, encoding schemes,
rate-limiting algorithms, input-validation pitfalls, and redirect semantics.
