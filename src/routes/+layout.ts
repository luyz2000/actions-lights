// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;

export const prerender = true;

import { dev } from "$app/environment";
import { invoke } from "@tauri-apps/api/core";
import { resolveLocale, setLocale } from "$lib/i18n";

export const load = async () => {
  if (typeof window === "undefined") return;
  if ("__TAURI_INTERNALS__" in window) {
    // Same language as the tray and notifications, picked by the backend.
    setLocale(await invoke<string>("get_locale").catch(() => resolveLocale(navigator.languages)));
  } else {
    setLocale(resolveLocale(navigator.languages));
    // In a plain browser during `npm run dev`, preview the UI against a fake backend.
    if (dev) (await import("$lib/mock")).install();
  }
};
