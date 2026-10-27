// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;

export const prerender = true;

import { dev } from "$app/environment";

// In a plain browser during `npm run dev`, preview the UI against a fake backend.
export const load = async () => {
  if (dev && typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window)) {
    (await import("$lib/mock")).install();
  }
};
