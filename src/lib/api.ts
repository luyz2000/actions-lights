import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Light = "gray" | "yellow" | "green" | "red";
export type TokenSource = "gh" | "manual" | "oauth" | "none";

export interface Card {
  key: string;
  repo: string;
  workflow_name: string;
  branch: string | null;
  light: Light;
  label: string | null;
  run_url: string | null;
  started_at: string | null;
  completed_at: string | null;
  progress: number | null;
  expected_secs: number | null;
  step_label: string | null;
  step_short: string | null;
  failed_step: string | null;
  error: string | null;
  run_branch: string | null;
  run_event: string | null;
  run_title: string | null;
  run_actor: string | null;
}

export interface RepoGroup {
  repo: string;
  cards: Card[];
}

export interface AuthStatus {
  source: TokenSource;
  login: string | null;
  gh: boolean;
  manual: boolean;
  oauth: boolean;
  oauth_available: boolean;
}

export interface DeviceCode {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface DashboardState {
  groups: RepoGroup[];
  auth: AuthStatus;
  banner: string | null;
  banner_url: string | null;
  last_updated: string | null;
}

export interface NotifyPrefs {
  on_success: boolean;
  on_failure: boolean;
  on_start: boolean;
}

export interface WatchedWorkflow {
  repo: string;
  workflow_id: number;
  workflow_name: string;
  branch: string | null;
  event: string | null;
  notify: NotifyPrefs;
}

export interface Config {
  watched: WatchedWorkflow[];
  autostart: boolean;
  sound: boolean;
  filters_cleared: boolean;
}

export interface Repo {
  full_name: string;
  private: boolean;
  default_branch: string;
}

export interface Workflow {
  id: number;
  name: string;
  path: string;
  state: string;
}

export interface UpdateInfo {
  version: string;
  notes: string | null;
}

export interface NotificationStatus {
  available: boolean;
  reason: string | null;
}

export const api = {
  getDashboard: () => invoke<DashboardState>("get_dashboard"),
  refreshNow: () => invoke<void>("refresh_now"),
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (cfg: Config) => invoke<void>("save_config", { cfg }),
  listRepos: () => invoke<Repo[]>("list_repos"),
  getRepo: (full: string) => invoke<Repo>("get_repo", { full }),
  listWorkflows: (repo: string) => invoke<Workflow[]>("list_workflows", { repo }),
  authStatus: () => invoke<AuthStatus>("auth_status"),
  setToken: (token: string) => invoke<AuthStatus>("set_token", { token }),
  clearToken: () => invoke<AuthStatus>("clear_token"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  repoFromFolder: (path: string) => invoke<Repo>("repo_from_folder", { path }),
  oauthStart: () => invoke<DeviceCode>("oauth_start"),
  oauthLogout: () => invoke<AuthStatus>("oauth_logout"),
  openSettings: () => invoke<void>("open_settings"),
  quit: () => invoke<void>("quit"),
  getUpdate: () => invoke<UpdateInfo | null>("get_update"),
  installUpdate: () => invoke<void>("install_update"),
  testNotification: () => invoke<void>("test_notification"),
  notificationsStatus: () => invoke<NotificationStatus>("notifications_status"),
};

export function onUpdate(cb: (u: UpdateInfo | null) => void): Promise<UnlistenFn> {
  return listen<UpdateInfo | null>("update", (e) => cb(e.payload));
}

export function onDashboard(cb: (d: DashboardState) => void): Promise<UnlistenFn> {
  return listen<DashboardState>("dashboard", (e) => cb(e.payload));
}

export function onAuthChanged(cb: (a: AuthStatus) => void): Promise<UnlistenFn> {
  return listen<AuthStatus>("auth-changed", (e) => cb(e.payload));
}

export function onOauthFailed(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("oauth-failed", (e) => cb(e.payload));
}
