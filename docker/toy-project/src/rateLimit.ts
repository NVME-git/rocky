import type { Request, Response, NextFunction } from "express";

interface Bucket {
  hits: number[];
}

export interface RateLimitOpts {
  windowMs: number;
  max: number;
}

export function rateLimit(opts: RateLimitOpts) {
  const buckets = new Map<string, Bucket>();

  return (req: Request, res: Response, next: NextFunction) => {
    const key = req.ip ?? "unknown";
    const now = Date.now();
    const cutoff = now - opts.windowMs;

    let bucket = buckets.get(key);
    if (!bucket) {
      bucket = { hits: [] };
      buckets.set(key, bucket);
    }
    bucket.hits = bucket.hits.filter((t) => t > cutoff);

    if (bucket.hits.length >= opts.max) {
      const oldest = bucket.hits[0];
      const retryMs = oldest + opts.windowMs - now;
      res.setHeader("Retry-After", Math.ceil(retryMs / 1000).toString());
      res.status(429).json({ error: "rate limit exceeded" });
      return;
    }

    bucket.hits.push(now);
    next();
  };
}
