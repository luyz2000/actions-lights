<script lang="ts">
  import { open as pickDir } from "@tauri-apps/plugin-dialog";
  import type { Repo } from "$lib/api";
  import { t } from "$lib/i18n";

  let {
    repos,
    selected,
    watchedRepos,
    loading,
    error,
    onselect,
    onaddpublic,
    onaddfolder,
    onreload,
  }: {
    repos: Repo[];
    selected: string | null;
    watchedRepos: Map<string, number>;
    loading: boolean;
    error: string | null;
    onselect: (repo: Repo) => void;
    onaddpublic: (full: string) => Promise<string | null>;
    onaddfolder: (path: string) => Promise<string | null>;
    onreload: () => void;
  } = $props();

  let filter = $state("");
  let name = $state("");
  let addError = $state<string | null>(null);

  const shown = $derived(
    repos
      .filter((r) => r.full_name.toLowerCase().includes(filter.trim().toLowerCase()))
      .sort((a, b) => Number(watchedRepos.has(b.full_name)) - Number(watchedRepos.has(a.full_name))),
  );

  async function addByName() {
    if (!/^[\w.-]+\/[\w.-]+$/.test(name.trim())) {
      addError = t("repos.invalid_name");
      return;
    }
    addError = await onaddpublic(name.trim());
    if (!addError) name = "";
  }

  async function addFolder() {
    const dir = await pickDir({ directory: true, title: t("repos.pick_folder_title") });
    if (typeof dir === "string") addError = await onaddfolder(dir);
  }
</script>

<div class="col">
  <div class="top">
    <input id="repo-search" class="field" placeholder={t("repos.search")} bind:value={filter} />
    <button class="btn" onclick={onreload} title={t("repos.reload")} aria-label={t("repos.reload_aria")}>
      <svg width="13" height="13" viewBox="0 0 16 16"><path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" /></svg>
    </button>
  </div>

  <div class="list">
    {#if loading}
      <p class="muted pad">{t("repos.loading")}</p>
    {:else if error}
      <p class="muted pad">{error}</p>
    {/if}
    {#each shown as repo (repo.full_name)}
      {@const n = watchedRepos.get(repo.full_name) ?? 0}
      <button class="row" class:sel={repo.full_name === selected} onclick={() => onselect(repo)}>
        <span class="name">{repo.full_name}</span>
        {#if n}<span class="count" title={t("repos.watched_count")}>{n}</span>{/if}
        {#if repo.private}<svg class="lock" width="11" height="11" viewBox="0 0 16 16" aria-label={t("repos.private")}><rect x="3" y="7" width="10" height="7" rx="1.5" fill="currentColor" /><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" fill="none" stroke="currentColor" stroke-width="1.6" /></svg>{/if}
      </button>
    {/each}
  </div>

  <div class="add">
    <div class="add-row">
      <input id="repo-name" class="field" placeholder="owner/repo" bind:value={name} onkeydown={(e) => e.key === "Enter" && addByName()} />
      <button class="btn" onclick={addByName}>{t("repos.add")}</button>
    </div>
    <button class="btn folder" onclick={addFolder}>
      <svg width="13" height="13" viewBox="0 0 16 16"><path d="M1.5 4.5a1 1 0 0 1 1-1h3.6l1.5 1.5h5.9a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" fill="none" stroke="currentColor" stroke-width="1.4" /></svg>
      {t("repos.pick_folder")}
    </button>
    {#if addError}<p class="error">{addError}</p>{/if}
  </div>
</div>

<style>
  .col {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 8px;
  }
  .top {
    display: flex;
    gap: 6px;
  }
  .top .field {
    flex: 1;
    min-width: 0;
  }
  .top .btn {
    display: grid;
    place-items: center;
    padding: 0 8px;
    color: var(--muted);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    background: var(--surface);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 4px;
  }
  .pad {
    padding: 6px 8px;
    margin: 0;
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
    width: 100%;
    text-align: left;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.sel {
    background: var(--selected);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: 11px;
    min-width: 16px;
    text-align: center;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-fg);
    font-variant-numeric: tabular-nums;
  }
  .lock {
    color: var(--muted);
    flex: none;
  }
  .add {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .add-row {
    display: flex;
    gap: 6px;
  }
  .add-row .field {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
  }
  .folder {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--red);
  }
</style>
