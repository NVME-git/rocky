const BLOCKED_SCHEMES = new Set(["javascript:", "data:", "vbscript:", "file:"]);

export interface ValidationError {
  ok: false;
  reason: string;
}
export interface ValidationOk {
  ok: true;
  url: string;
}
export type ValidationResult = ValidationOk | ValidationError;

export function validateUrl(raw: unknown): ValidationResult {
  if (typeof raw !== "string") {
    return { ok: false, reason: "url must be a string" };
  }
  const trimmed = raw.trim();
  if (trimmed.length === 0) return { ok: false, reason: "url is empty" };
  if (trimmed.length > 2048) return { ok: false, reason: "url exceeds 2048 chars" };

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return { ok: false, reason: "url is not parseable" };
  }

  const scheme = parsed.protocol.toLowerCase();
  if (BLOCKED_SCHEMES.has(scheme)) {
    return { ok: false, reason: `scheme ${scheme} is not allowed` };
  }
  if (scheme !== "http:" && scheme !== "https:") {
    return { ok: false, reason: `only http and https are allowed, got ${scheme}` };
  }
  return { ok: true, url: parsed.toString() };
}
