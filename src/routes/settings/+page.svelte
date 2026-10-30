<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import "$lib/theme.css";
  import { api, type AuthStatus, type Config, type NotificationStatus, type Repo, type WatchedWorkflow, type Workflow } from "$lib/api";
  import Account from "./Account.svelte";
  import RepoList from "./RepoList.svelte";
  import WorkflowList from "./WorkflowList.svelte";

  type Section = "repos" | "account" | "general";

  let section = $state<Section>("repos");
  let cfg = $state<Config>({ watched: [], autostart: true, sound: true, filters_cleared: true });
  let auth = $state<AuthStatus | null>(null);
  let notifications = $state<NotificationStatus | null>(null);
  let repos = $state<Repo[]>([]);
  let reposLoading = $state(true);
  let reposError = $state<string | null>(null);
  let selected = $state<Repo | null>(null);
  let workflows = $state<Record<string, Workflow[]>>({});
  let wfLoading = $state(false);
  let wfError = $state<string | null>(null);
  let saved = $state<"ok" | string | null>(null);

  const watchedRepos = $derived.by(() => {
    const m = new Map<string, number>();
    for (const w of cfg.watched) m.set(w.repo, (m.get(w.repo) ?? 0) + 1);
    return m;
  });

  async function loadRepos() {
    reposLoading = true;
    reposError = null;
    let list: Repo[] = [];
    try {
      list = await api.listRepos();
    } catch (e) {
      reposError =
        auth?.source === "none"
          ? "Sin cuenta conectada no podemos listar tus repos. Conecta una en Cuenta o añade un repo público abajo."
          : String(e);
    }
    // Watched repos that the listing doesn't include (e.g. public repos of others).
    for (const full of watchedRepos.keys()) {
      if (!list.some((r) => r.full_name === full)) {
        list.push(await api.getRepo(full).catch(() => ({ full_name: full, private: false, default_branch: "main" })));
      }
    }
    repos = list;
    reposLoading = false;
    if (!selected && list.length) select(list.find((r) => watchedRepos.has(r.full_name)) ?? list[0]);
  }

  async function select(repo: Repo) {
    selected = repo;
    if (workflows[repo.full_name]) return;
    wfLoading = true;
    wfError = null;
    try {
      workflows[repo.full_name] = await api.listWorkflows(repo.full_name);
    } catch (e) {
      wfError = String(e);
    } finally {
      wfLoading = false;
    }
  }

  async function addRepo(load: () => Promise<Repo>): Promise<string | null> {
    try {
      const repo = await load();
      if (!repos.some((r) => r.full_name === repo.full_name)) repos = [repo, ...repos];
      await select(repo);
      return null;
    } catch (e) {
      return String(e);
    }
  }

  function changeWatched(repo: string, workflowId: number, entry: WatchedWorkflow | null) {
    const i = cfg.watched.findIndex((w) => w.repo === repo && w.workflow_id === workflowId);
    if (entry && i >= 0) cfg.watched[i] = entry;
    else if (entry) cfg.watched.push(entry);
    else if (i >= 0) cfg.watched.splice(i, 1);
    save();
  }

  // Every change is saved right away; there is no "Guardar" step to forget.
  async function save() {
    try {
      await api.saveConfig($state.snapshot(cfg));
      saved = "ok";
    } catch (e) {
      saved = `No se pudo guardar: ${e}`;
    }
  }

  async function onAuth(a: AuthStatus) {
    auth = a;
    workflows = {};
    selected = null;
    await loadRepos();
  }

  // The window lives hidden for the whole session: refresh what may have changed since
  // launch (gh login finished, network came up) every time it's brought forward.
  async function refresh() {
    auth = await api.authStatus();
    if (reposError || repos.length === 0) await loadRepos();
  }

  onMount(() => {
    api.notificationsStatus().then((n) => (notifications = n));
    Promise.all([api.getConfig(), api.authStatus()]).then(async ([c, a]) => {
      cfg = c;
      auth = a;
      if (a.source === "none" && c.watched.length === 0) section = "account";
      await loadRepos();
    });
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });
    return () => {
      unlisten.then((f) => f());
    };
  });

  const NAV: { id: Section; label: string; icon: string }[] = [
    { id: "repos", label: "Repos y workflows", icon: "M2 3.5h12M2 8h12M2 12.5h7" },
    { id: "account", label: "Cuenta", icon: "M8 8a2.8 2.8 0 1 0 0-5.6A2.8 2.8 0 0 0 8 8zm-5 5.6c.6-2.4 2.6-3.8 5-3.8s4.4 1.4 5 3.8" },
    { id: "general", label: "General", icon: "M3 4.5h10M3 11.5h10M6 2.8v3.4M10 9.8v3.4" },
  ];
</script>

<div class="window">
  <nav>
    <div class="brand">
      <svg class="logo" width="22" height="22" viewBox="100 100 824 824" aria-hidden="true">
        <rect x="103" y="103" width="818" height="818" rx="183" fill="#F6F8FA" stroke="#D0D7DE" stroke-width="6" />
        <path d="M262 400H762M512 400V680" stroke="#8B949E" stroke-width="40" />
        <circle cx="262" cy="400" r="100" fill="#2EA043" />
        <circle cx="512" cy="400" r="100" fill="#2EA043" />
        <path d="M220 400l30 32 57-64M470 400l30 32 57-64" fill="none" stroke="#FFFFFF" stroke-width="28" stroke-linecap="round" stroke-linejoin="round" />
        <circle cx="762" cy="400" r="82" fill="#F6F8FA" stroke="#D29922" stroke-width="36" />
        <circle cx="762" cy="400" r="34" fill="#D29922" />
        <circle cx="512" cy="680" r="100" fill="#F85149" />
        <path d="M476 644L548 716M548 644L476 716" stroke="#FFFFFF" stroke-width="28" stroke-linecap="round" />
      </svg>
      Actions Lights
    </div>
    {#each NAV as item (item.id)}
      <button class="nav-item" class:active={section === item.id} onclick={() => (section = item.id)}>
        <svg width="15" height="15" viewBox="0 0 16 16"><path d={item.icon} fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
        <span>{item.label}</span>
        {#if item.id === "repos" && cfg.watched.length}<span class="badge">{cfg.watched.length}</span>{/if}
        {#if item.id === "account" && auth?.source === "none"}<span class="dot-warn" title="Sin cuenta"></span>{/if}
      </button>
    {/each}
    <div class="nav-foot muted">
      {#if saved === "ok"}Cambios guardados{:else if saved}<span class="error">{saved}</span>{/if}
    </div>
  </nav>

  <main>
    {#if section === "repos"}
      <div class="repos">
        <header class="page-head">
          <h1>Repos y workflows</h1>
          <p class="muted">Activa los workflows que quieres ver en la barra. Normalmente, el deploy de tu rama de release.</p>
        </header>
        <div class="cols">
          <div class="left">
            <RepoList
              {repos}
              selected={selected?.full_name ?? null}
              {watchedRepos}
              loading={reposLoading}
              error={reposError}
              onselect={select}
              onaddpublic={(full) => addRepo(() => api.getRepo(full))}
              onaddfolder={(path) => addRepo(() => api.repoFromFolder(path))}
              onreload={loadRepos}
            />
          </div>
          <div class="right">
            {#if selected}
              <WorkflowList
                repo={selected}
                workflows={workflows[selected.full_name] ?? []}
                watched={cfg.watched}
                loading={wfLoading}
                error={wfError}
                onchange={(id, entry) => changeWatched(selected!.full_name, id, entry)}
                notificationsAvailable={notifications?.available !== false}
              />
            {:else}
              <p class="muted hint">Elige un repo para ver sus workflows.</p>
            {/if}
          </div>
        </div>
      </div>
    {:else if section === "account"}
      <div class="scroll"><Account {auth} onauth={onAuth} /></div>
    {:else}
      <div class="scroll general">
        <header class="page-head">
          <h1>General</h1>
        </header>
        <div class="group">
          <label class="setting" for="autostart">
            <div>
              <div class="label">Abrir al iniciar sesión</div>
              <div class="muted small">La app arranca sola y vive en la barra de menús.</div>
            </div>
            <input id="autostart" class="switch" type="checkbox" bind:checked={cfg.autostart} onchange={save} />
          </label>
          <label class="setting" for="sound">
            <div>
              <div class="label">Sonido en notificaciones {#if notifications?.available === false}<span class="soon">Próximamente</span>{/if}</div>
              <div class="muted small">Un sonido distinto cuando un workflow termina bien y cuando falla.</div>
            </div>
            <input id="sound" class="switch" type="checkbox" bind:checked={cfg.sound} onchange={save} disabled={notifications?.available === false} />
          </label>
          <div class="setting">
            <div>
              <div class="label">Probar notificaciones {#if notifications?.available === false}<span class="soon">Próximamente</span>{/if}</div>
              <div class="muted small">Envía una notificación de ejemplo; al pulsarla se abre el enlace en GitHub.</div>
            </div>
            <button class="btn" disabled={notifications?.available === false} onclick={() => api.testNotification()}>Enviar notificación de prueba</button>
          </div>
          <div class="setting">
            <div>
              <div class="label">Frecuencia de consulta</div>
              <div class="muted small">Cada 15 s si algo está corriendo, cada 60 s en reposo y cada 5 min sin cuenta. Se espacia sola si el límite de la API baja.</div>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>

<style>
  :global(html),
  :global(body) {
    height: 100%;
    background: var(--bg);
  }
  .window {
    height: 100%;
    display: flex;
  }
  nav {
    width: 200px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 10px;
    border-right: 0.5px solid var(--border);
    background: var(--surface-2);
    box-sizing: border-box;
  }
  :global(.soon) {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 10.5px;
    font-weight: 600;
    vertical-align: middle;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    padding: 2px 8px 14px;
  }
  .logo {
    flex: none;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--fg);
  }
  .nav-item svg {
    color: var(--muted);
  }
  .nav-item:hover {
    background: var(--hover);
  }
  .nav-item.active {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .nav-item.active svg {
    color: var(--accent-fg);
  }
  .badge {
    margin-left: auto;
    font-size: 11px;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--hover);
    font-variant-numeric: tabular-nums;
  }
  .dot-warn {
    margin-left: auto;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--yellow);
  }
  .nav-foot {
    margin-top: auto;
    font-size: 11px;
    padding: 0 8px;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .page-head h1 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .page-head p {
    margin: 2px 0 0;
    font-size: 12px;
  }
  .repos {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
  }
  .left {
    width: 230px;
    flex: none;
  }
  .right {
    flex: 1;
    min-width: 0;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
  }
  .general {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .group {
    background: var(--surface);
    border-radius: 10px;
    box-shadow: var(--shadow);
  }
  .setting {
    display: flex;
    align-items: center;
    gap: 16px;
    justify-content: space-between;
    padding: 12px 14px;
  }
  .setting + .setting {
    border-top: 0.5px solid var(--border);
  }
  .label {
    font-weight: 500;
  }
  .small {
    font-size: 12px;
  }
  .hint {
    margin-top: 60px;
    text-align: center;
  }
  .error {
    color: var(--red);
  }
</style>
