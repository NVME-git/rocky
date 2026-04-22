import { describe, expect, it } from "vitest";
import { encodeBase62, decodeBase62 } from "../src/encode";

describe("base62", () => {
  it("round-trips small ints", () => {
    for (const n of [0, 1, 61, 62, 1234, 999_999_999]) {
      expect(decodeBase62(encodeBase62(n))).toBe(n);
    }
  });

  it("rejects negatives", () => {
    expect(() => encodeBase62(-1)).toThrow();
  });

  it("rejects invalid chars on decode", () => {
    expect(() => decodeBase62("a!b")).toThrow();
  });
});
