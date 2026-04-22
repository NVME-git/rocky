const ALPHABET =
  "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

export function encodeBase62(n: number): string {
  if (n < 0 || !Number.isInteger(n)) {
    throw new Error(`encodeBase62: expected non-negative integer, got ${n}`);
  }
  if (n === 0) return ALPHABET[0];
  let out = "";
  while (n > 0) {
    out = ALPHABET[n % 62] + out;
    n = Math.floor(n / 62);
  }
  return out;
}

export function decodeBase62(s: string): number {
  let n = 0;
  for (const ch of s) {
    const idx = ALPHABET.indexOf(ch);
    if (idx < 0) throw new Error(`decodeBase62: invalid char '${ch}'`);
    n = n * 62 + idx;
  }
  return n;
}
