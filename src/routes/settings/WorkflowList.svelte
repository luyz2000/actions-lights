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
  const ANY = "";

  function entryFor(id: number): WatchedWorkflow | undefined {
    return watched.find((w) => w.repo === repo.full_name && w.workflow_id === id);
  }

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

  function update(entry: WatchedWorkflow, patch: Partial<WatchedWorkflow>) {
    onchange(entry.workflow_id, { ...entry, ...patch });
  }

  function setNotify(entry: WatchedWorkflow, key: keyof NotifyPrefs, value: boolean) {
    update(entry, { notify: { ...entry.notify, [key]: value } });
  }
</script>

<div class="col">
  <div class="title"><b>{repo.full_name}</b> <span class="muted">— workflows</span></div>
  {#if loading}
    <p class="muted">Cargando workflows…</p>
  {:else if error}
    <p class="err">{error}</p>
  {:else if workflows.length === 0}
    <p class="muted">Este repo no tiene workflows de GitHub Actions.</p>
  {/if}
  <div class="list">
    {#each workflows as wf (wf.id)}
      {@const entry = entryFor(wf.id)}
      <div class="wf" class:on={!!entry}>
        <label class="head">
          <input type="checkbox" checked={!!entry} onchange={(e) => toggle(wf, e.currentTarget.checked)} />
          <span>{wf.name}</span>
          {#if wf.state !== "active"}<span class="muted small">(desactivado)</span>{/if}
        </label>
        {#if entry}
          <div class="opts">
            <label>
              Rama
              <input
                class="branch"
                placeholder="cualquiera"
                value={entry.branch ?? ""}
                onchange={(e) => update(entry, { branch: e.currentTarget.value.trim() || null })}
              />
            </label>
            <label>
              Evento
              <select
                value={entry.event ?? ANY}
                onchange={(e) => update(entry, { event: e.currentTarget.value || null })}
              >
                {#each EVENTS as ev (ev)}<option value={ev}>{ev}</option>{/each}
                <option value={ANY}>cualquiera</option>
              </select>
            </label>
          </div>
          <div class="opts notify">
            Notificar:
            <label><input type="checkbox" checked={entry.notify.on_success} onchange={(e) => setNotify(entry, "on_success", e.currentTarget.checked)} /> al terminar bien</label>
            <label><input type="checkbox" checked={entry.notify.on_failure} onchange={(e) => setNotify(entry, "on_failure", e.currentTarget.checked)} /> al fallar</label>
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
  }
  .title {
    margin-bottom: 8px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .wf {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    margin-bottom: 8px;
  }
  .wf.on {
    border-color: var(--accent);
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
  }
  .opts {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: 8px;
    font-size: 12px;
    color: var(--muted);
    align-items: center;
  }
  .opts label {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .branch,
  select {
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--input);
    color: var(--fg);
    font: inherit;
  }
  .branch {
    width: 120px;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 11px;
  }
  .err {
    color: #d1242f;
  }
</style>
