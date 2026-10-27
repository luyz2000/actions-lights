import { describe, expect, it } from "vitest";
import { formatAgo, formatDuration } from "./format";

describe("formatDuration", () => {
  it("formats seconds, minutes and hours", () => {
    expect(formatDuration(45)).toBe("45s");
    expect(formatDuration(192)).toBe("3m 12s");
    expect(formatDuration(3720)).toBe("1h 2m");
  });

  it("clamps negatives to zero", () => {
    expect(formatDuration(-5)).toBe("0s");
  });
});

describe("formatAgo", () => {
  const now = new Date("2026-10-27T12:00:00Z");
  const minus = (secs: number) => new Date(now.getTime() - secs * 1000).toISOString();

  it("uses the largest unit", () => {
    expect(formatAgo(minus(30), now)).toBe("hace un momento");
    expect(formatAgo(minus(5 * 60), now)).toBe("hace 5m");
    expect(formatAgo(minus(2 * 3600), now)).toBe("hace 2h");
    expect(formatAgo(minus(3 * 86400), now)).toBe("hace 3d");
  });
});
