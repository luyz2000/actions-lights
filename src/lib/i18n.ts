// UI text in the system language. The catalogs are shared with the Rust side
// (src-tauri/src/i18n.rs), which also decides the language at launch.
import en from "./i18n/en.json";
import es from "./i18n/es.json";
import fr from "./i18n/fr.json";
import ja from "./i18n/ja.json";
import ru from "./i18n/ru.json";
import zh from "./i18n/zh.json";

export type Locale = "en" | "es" | "ru" | "fr" | "ja" | "zh";
type Vars = Record<string, string | number>;

export const CATALOGS: Record<Locale, Record<string, string>> = { en, es, ru, fr, ja, zh };

/** BCP 47 tags of the variants each catalog is written in, for Intl and `<html lang>`. */
const TAGS: Record<Locale, string> = { en: "en-US", es: "es-MX", ru: "ru", fr: "fr", ja: "ja", zh: "zh-CN" };

/** Each language in its own name, for the language picker. */
export const LANGUAGE_NAMES: Record<Locale, string> = {
  en: "English",
  es: "Español",
  ru: "Русский",
  fr: "Français",
  ja: "日本語",
  zh: "中文（简体）",
};

let current: Locale = "en";
let plurals = new Intl.PluralRules(TAGS.en);

/** First supported language among the system's preferred ones ("es-MX", "zh_CN"…), else English. */
export function resolveLocale(preferred: readonly string[]): Locale {
  for (const tag of preferred) {
    const lang = tag.split(/[-_]/)[0].toLowerCase();
    if (lang in CATALOGS) return lang as Locale;
  }
  return "en";
}

export function setLocale(code: string) {
  current = code in CATALOGS ? (code as Locale) : "en";
  plurals = new Intl.PluralRules(TAGS[current]);
  if (typeof document !== "undefined") document.documentElement.lang = TAGS[current];
}

function fill(text: string, vars?: Vars): string {
  return vars ? text.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : text;
}

export function t(key: string, vars?: Vars): string {
  return fill(CATALOGS[current][key] ?? CATALOGS.en[key] ?? key, vars);
}

/** Plural-aware text: looks up `key.one`, `key.few`… per the language's rules; `{count}` is filled in. */
export function tn(key: string, count: number, vars?: Vars): string {
  const catalog = CATALOGS[current];
  const text = catalog[`${key}.${plurals.select(count)}`] ?? catalog[`${key}.other`] ?? CATALOGS.en[`${key}.other`] ?? key;
  return fill(text, { count, ...vars });
}

/** Splits a text around one placeholder, to wrap the value in markup (e.g. a bold login). */
export function tSplit(key: string, name: string): [string, string] {
  const [before, after = ""] = t(key).split(`{${name}}`);
  return [before, after];
}
