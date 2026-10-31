import { afterEach, describe, expect, it } from "vitest";
import { CATALOGS, resolveLocale, setLocale, t, tn } from "./i18n";
import { formatAgo } from "./format";

const PLURAL = /\.(zero|one|two|few|many|other)$/;
const base = (key: string) => key.replace(PLURAL, "");
const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("catalogs", () => {
  const en = CATALOGS.en;
  const enBase = new Set(Object.keys(en).map(base));

  for (const [code, catalog] of Object.entries(CATALOGS)) {
    it(`${code} has exactly the English keys`, () => {
      expect(new Set(Object.keys(catalog).map(base))).toEqual(enBase);
      for (const key of Object.keys(catalog).filter((k) => PLURAL.test(k))) {
        expect(catalog, `${key} needs an .other form`).toHaveProperty(`${base(key)}.other`);
      }
    });

    it(`${code} keeps the English placeholders`, () => {
      for (const [key, text] of Object.entries(catalog)) {
        const reference = en[key] ?? en[`${base(key)}.other`];
        expect(placeholders(text), key).toEqual(placeholders(reference));
      }
    });
  }
});

describe("i18n", () => {
  afterEach(() => setLocale("es"));

  it("picks the first supported system language, English otherwise", () => {
    expect(resolveLocale(["es-MX", "en-US"])).toBe("es");
    expect(resolveLocale(["de-DE", "fr-CA"])).toBe("fr");
    expect(resolveLocale(["zh-Hans-CN"])).toBe("zh");
    expect(resolveLocale(["ja_JP"])).toBe("ja");
    expect(resolveLocale(["de-DE"])).toBe("en");
    expect(resolveLocale([])).toBe("en");
  });

  it("fills placeholders", () => {
    setLocale("en");
    expect(t("update.available", { version: "1.6.0" })).toBe("Version 1.6.0 available");
  });

  it("falls back to an unknown locale's English text", () => {
    setLocale("xx");
    expect(t("popover.quit")).toBe("Quit");
  });

  it("uses each language's plural forms", () => {
    setLocale("en");
    expect(tn("popover.failed", 1)).toBe("1 workflow failed");
    expect(tn("popover.failed", 3)).toBe("3 workflows failed");
    setLocale("ru");
    expect(tn("popover.running", 1)).toBe("1 workflow выполняется");
    expect(tn("popover.running", 3)).toBe("3 workflow выполняются");
    setLocale("ja");
    expect(tn("popover.running", 1)).toBe("1件のワークフローが実行中");
  });

  it("formats relative times in the current language", () => {
    const now = new Date("2026-10-31T12:00:00Z");
    setLocale("en");
    expect(formatAgo("2026-10-31T11:55:00Z", now)).toBe("5m ago");
    setLocale("fr");
    expect(formatAgo("2026-10-31T09:00:00Z", now)).toBe("il y a 3 h");
  });
});
