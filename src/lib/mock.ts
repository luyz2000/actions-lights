// Dev-only fake backend so the UI can be previewed in a plain browser (`npm run dev`).
// Never loaded inside the Tauri app.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AuthStatus, Config, DashboardState } from "./api";

const ago = (s: number) => new Date(Date.now() - s * 1000).toISOString();

const dashboard: DashboardState = {
  auth: { source: "gh", login: "luyz2000", gh: true, manual: false, oauth: false, oauth_available: true },
  banner: null,
  banner_url: null,
  last_updated: ago(3),
  groups: [
    {
      repo: "acme/api",
      cards: [
        {
          key: "a1", repo: "acme/api", workflow_name: "Deploy", branch: "release", light: "yellow", label: null,
          run_url: "https://github.com", started_at: ago(192), completed_at: null, progress: 0.62, expected_secs: 310,
          step_label: "Paso 5/9 · Build image", failed_step: null, error: null,
        },
        {
          key: "a2", repo: "acme/api", workflow_name: "Tests", branch: "release", light: "green", label: null,
          run_url: "https://github.com", started_at: ago(700), completed_at: ago(300), progress: null, expected_secs: null,
          step_label: null, failed_step: null, error: null,
        },
      ],
    },
    {
      repo: "acme/web",
      cards: [
        {
          key: "w1", repo: "acme/web", workflow_name: "Deploy", branch: "main", light: "red", label: null,
          run_url: "https://github.com", started_at: ago(1500), completed_at: ago(1200), progress: null, expected_secs: null,
          step_label: null, failed_step: "Run migrations", error: null,
        },
        {
          key: "w2", repo: "acme/web", workflow_name: "Release", branch: null, light: "yellow", label: "esperando aprobación",
          run_url: "https://github.com", started_at: ago(40), completed_at: null, progress: null, expected_secs: null,
          step_label: null, failed_step: null, error: null,
        },
      ],
    },
  ],
};

const config: Config = {
  autostart: true,
  watched: [
    { repo: "acme/api", workflow_id: 1, workflow_name: "Deploy", branch: "release", event: "push", notify: { on_success: true, on_failure: true, on_start: false } },
  ],
};

export function install() {
  const params = new URLSearchParams(location.search);
  const auth: AuthStatus = { ...dashboard.auth };
  if (params.has("nogh")) Object.assign(auth, { source: "none", login: null, gh: false });
  if (params.has("banner")) {
    dashboard.banner = "Autoriza el token para la organización (SSO) — clic para abrir";
    dashboard.banner_url = "https://github.com";
  }
  if (params.has("empty")) dashboard.groups = [];

  mockWindows(location.pathname.includes("settings") ? "settings" : "popover");
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "get_dashboard": return dashboard;
      case "get_config": return config;
      case "auth_status": return auth;
      case "list_repos":
        return [
          { full_name: "acme/api", private: true, default_branch: "main" },
          { full_name: "acme/web", private: true, default_branch: "main" },
          { full_name: "luyz2000/actions-lights", private: false, default_branch: "main" },
          { full_name: "luyz2000/dotfiles", private: false, default_branch: "master" },
        ];
      case "list_workflows":
        return [
          { id: 1, name: "Deploy", path: ".github/workflows/deploy.yml", state: "active" },
          { id: 2, name: "Tests", path: ".github/workflows/tests.yml", state: "active" },
          { id: 3, name: "CodeQL", path: ".github/workflows/codeql.yml", state: "disabled_manually" },
        ];
      case "oauth_start":
        return { device_code: "x", user_code: "WDJB-MJHT", verification_uri: "https://github.com/login/device", expires_in: 900, interval: 5 };
      case "plugin:event|listen": return Math.floor(Math.random() * 1e6);
      case "plugin:dialog|open": return "/Users/luis2000/repositorios/acme-api";
      case "repo_from_folder": return { full_name: "acme/api", private: true, default_branch: "main" };
      default:
        console.debug("mock ipc", cmd, args);
        return null;
    }
  });
}
