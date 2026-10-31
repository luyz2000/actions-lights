import type { Card } from "./api";
import { t } from "./i18n";


export function formatDuration(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

export function formatAgo(iso: string, now: Date): string {
  const secs = Math.max(0, (now.getTime() - new Date(iso).getTime()) / 1000);
  if (secs < 60) return t("ago.now");
  if (secs < 3600) return t("ago.minutes", { n: Math.floor(secs / 60) });
  if (secs < 86400) return t("ago.hours", { n: Math.floor(secs / 3600) });
  return t("ago.days", { n: Math.floor(secs / 86400) });
}

/** Most recent completion among successful runs (ISO strings compare chronologically). */
export function latestFinish(cards: Card[]): string | null {
  let latest: string | null = null;
  for (const c of cards) {
    if (c.light === "green" && c.completed_at && (!latest || c.completed_at > latest)) latest = c.completed_at;
  }
  return latest;
}
