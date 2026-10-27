<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AuthStatus, type Config, type Repo, type WatchedWorkflow, type Workflow } from "$lib/api";
  import Account from "./Account.svelte";
  import RepoList from "./RepoList.svelte";
  import WorkflowList from "./WorkflowList.svelte";

  let cfg = $state<Config>({ watched: [], autostart: true });
  let auth = $state<AuthStatus | null>(null);
  let repos = $state<Repo[]>([]);
  let reposLoading = $state(true);
  let reposError = $state<string | null>(null);
  let selected = $state<Repo | null>(null);
  let workflows = $state<Record<string, Workflow[]>>({});
  let wfLoading = $state(false);
  let wfError = $state<string | null>(null);
  let status = $state<string | null>(null);

  const watchedRepos = $derived(new Set(cfg.watched.map((w) => w.repo)));

  async function loadRepos() {
    reposLoading = true;
    reposError = null;
    let list: Repo[] = [];
    try {
      list = await api.listRepos();
    } catch (e) {
      reposError = auth?.source === "none" ? "Sin token solo puedes añadir repos públicos abajo." : String(e);
    }
    // Watched repos that the listing doesn't include (e.g. public repos of others).
    for (const full of new Set(cfg.watched.map((w) => w.repo))) {
      if (!list.some((r) => r.full_name === full)) {
        list.push(await api.getRepo(full).catch(() => ({ full_name: full, private: false, default_branch: "main" })));
      }
    }
    repos = list;
    reposLoading = false;
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

  async function addPublic(full: string): Promise<string | null> {
    try {
      const repo = await api.getRepo(full);
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
    status = null;
  }

  async function save() {
    try {
      await api.saveConfig($state.snapshot(cfg));
      status = "Guardado ✓";
    } catch (e) {
      status = `No se pudo guardar: ${e}`;
    }
  }

  async function onAuth(a: AuthStatus) {
    auth = a;
    workflows = {};
    await loadRepos();
  }

  onMount(async () => {
    [cfg, auth] = await Promise.all([api.getConfig(), api.authStatus()]);
    await loadRepos();
  });
</script>

<main>
  <section class="cols">
    <div class="left">
      <RepoList
        {repos}
        selected={selected?.full_name ?? null}
        {watchedRepos}
        loading={reposLoading}
        error={reposError}
        onselect={select}
        onaddpublic={addPublic}
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
        />
      {:else}
        <p class="muted hint">Elige un repo a la izquierda para ver sus workflows.</p>
      {/if}
    </div>
  </section>

  <section class="bottom">
    <Account {auth} onauth={onAuth} />
    <div class="actions">
      <label><input type="checkbox" bind:checked={cfg.autostart} /> Abrir al iniciar sesión</label>
      <span class="muted">{cfg.watched.length} workflow(s) vigilados</span>
      {#if status}<span class="status">{status}</span>{/if}
      <button class="primary" onclick={save}>Guardar</button>
    </div>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
  }
  main {
    --bg: #ffffff;
    --fg: #1f2328;
    --muted: #656d76;
    --border: #d0d7de;
    --input: #ffffff;
    --hover: #f3f4f6;
    --selected: #e7f0ff;
    --accent: #0969da;
    box-sizing: border-box;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    font: 13px -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, sans-serif;
  }
  @media (prefers-color-scheme: dark) {
    main {
      --bg: #1e1e22;
      --fg: #e6e6e6;
      --muted: #9aa0a6;
      --border: #3a3a42;
      --input: #2b2b31;
      --hover: #2b2b31;
      --selected: #23324d;
      --accent: #4c8dff;
    }
  }
  .cols {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .left {
    width: 42%;
    padding: 12px;
    border-right: 1px solid var(--border);
    box-sizing: border-box;
  }
  .right {
    flex: 1;
    padding: 12px;
    min-width: 0;
  }
  .bottom {
    border-top: 1px solid var(--border);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .actions {
    display: flex;
    gap: 14px;
    align-items: center;
  }
  .actions .primary {
    margin-left: auto;
  }
  .primary {
    background: var(--accent);
    color: white;
    border: 0;
    border-radius: 6px;
    padding: 6px 14px;
    font: inherit;
    cursor: pointer;
  }
  .muted {
    color: var(--muted);
  }
  .hint {
    margin-top: 40px;
    text-align: center;
  }
  .status {
    color: #1a7f37;
  }
</style>
