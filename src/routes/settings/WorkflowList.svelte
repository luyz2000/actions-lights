<script lang="ts">
  import type { NotifyPrefs, Repo, WatchedWorkflow, Workflow } from "$lib/api";

  let {
    repo,
    workflows,
    watched,
    loading,
    error,
    onchange,
  }: {
    repo: Repo;
    workflows: Workflow[];
    watched: WatchedWorkflow[];
    loading: boolean;
    error: string | null;
    onchange: (workflowId: number, entry: WatchedWorkflow | null) => void;
  } = $props();

  const EVENTS = ["push", "pull_request", "workflow_dispatch", "schedule"];

  const entryFor = (id: number) => watched.find((w) => w.repo === repo.full_name && w.workflow_id === id);

  function toggle(wf: Workflow, on: boolean) {
    onchange(
      wf.id,
      on
        ? {
            repo: repo.full_name,
            workflow_id: wf.id,
            workflow_name: wf.name,
            branch: repo.default_branch,
            event: "push",
            notify: { on_success: true, on_failure: true, on_start: false },
          }
        : null,
    );
  }

  const update = (e: WatchedWorkflow, patch: Partial<WatchedWorkflow>) => onchange(e.workflow_id, { ...e, ...patch });
  const setNotify = (e: WatchedWorkflow, key: keyof NotifyPrefs, value: boolean) =>
    update(e, { notify: { ...e.notify, [key]: value } });
</script>

<div class="col">
  <div class="title">
    <h2>{repo.full_name}</h2>
    <span class="muted">Rama por defecto <span class="branch">{repo.default_branch}</span></span>
  </div>

  {#if loading}
    <p class="muted">Cargando workflows…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if workflows.length === 0}
    <p class="muted">Este repo no tiene workflows de GitHub Actions.</p>
  {/if}

  <div class="list">
    {#each workflows as wf (wf.id)}
      {@const entry = entryFor(wf.id)}
      <div class="wf" class:on={!!entry}>
        <label class="head" for="wf-{wf.id}">
          <div class="names">
            <span class="name">{wf.name}</span>
            <span class="path">{wf.path.replace(".github/workflows/", "")}</span>
          </div>
          {#if wf.state !== "active"}<span class="muted small">desactivado</span>{/if}
          <input id="wf-{wf.id}" class="switch" type="checkbox" checked={!!entry} onchange={(e) => toggle(wf, e.currentTarget.checked)} />
        </label>
        {#if entry}
          <div class="opts">
            <label for="br-{wf.id}">
              <span>Rama</span>
              <input
                id="br-{wf.id}"
                class="field mono"
                placeholder="cualquiera"
                value={entry.branch ?? ""}
                onchange={(e) => update(entry, { branch: e.currentTarget.value.trim() || null })}
              />
            </label>
            <label for="ev-{wf.id}">
              <span>Evento</span>
              <select id="ev-{wf.id}" class="field" value={entry.event ?? ""} onchange={(e) => update(entry, { event: e.currentTarget.value || null })}>
                {#each EVENTS as ev (ev)}<option value={ev}>{ev}</option>{/each}
                <option value="">cualquiera</option>
              </select>
            </label>
          </div>
          <div class="notify">
            <span class="muted">Avisarme</span>
            <label><input type="checkbox" checked={entry.notify.on_success} onchange={(e) => setNotify(entry, "on_success", e.currentTarget.checked)} /> al terminar bien</label>
            <label><input type="checkbox" checked={entry.notify.on_failure} onchange={(e) => setNotify(entry, "on_failure", e.currentTarget.checked)} /> si falla</label>
            <label><input type="checkbox" checked={entry.notify.on_start} onchange={(e) => setNotify(entry, "on_start", e.currentTarget.checked)} /> al empezar</label>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .col {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 10px;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .title .muted {
    font-size: 12px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .wf {
    background: var(--surface);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 600;
  }
  .path {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  .opts {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    padding-top: 10px;
    border-top: 0.5px solid var(--border);
  }
  .opts label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 11px;
    color: var(--muted);
  }
  .opts .field {
    width: 140px;
    font-size: 12px;
  }
  .mono {
    font-family: var(--mono);
  }
  .notify {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 12px;
    align-items: center;
  }
  .notify label {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .small {
    font-size: 11px;
  }
  .error {
    color: var(--red);
  }
</style>
