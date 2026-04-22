import express from "express";
import { openStore } from "./store";
import { encodeBase62, decodeBase62 } from "./encode";
import { validateUrl } from "./validate";
import { rateLimit } from "./rateLimit";

const PORT = Number(process.env.PORT ?? 3000);
const DB_PATH = process.env.DB_PATH ?? "./urlshort.db";

const store = openStore(DB_PATH);
const app = express();
app.use(express.json({ limit: "32kb" }));

app.get("/health", (_req, res) => {
  res.json({ ok: true });
});

app.post(
  "/shorten",
  rateLimit({ windowMs: 60_000, max: 10 }),
  (req, res) => {
    const result = validateUrl(req.body?.url);
    if (!result.ok) {
      res.status(400).json({ error: result.reason });
      return;
    }
    const id = store.insert(result.url);
    res.json({ short: encodeBase62(id) });
  }
);

app.get("/:id", (req, res) => {
  let id: number;
  try {
    id = decodeBase62(req.params.id);
  } catch {
    res.status(400).json({ error: "bad short id" });
    return;
  }
  const url = store.lookup(id);
  if (!url) {
    res.status(404).json({ error: "not found" });
    return;
  }
  res.redirect(302, url);
});

app.listen(PORT, () => {
  console.log(`urlshort listening on :${PORT}`);
});
