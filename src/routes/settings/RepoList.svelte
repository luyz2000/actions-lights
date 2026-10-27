<script lang="ts">
  import type { Repo } from "$lib/api";

  let {
    repos,
    selected,
    watchedRepos,
    loading,
    error,
    onselect,
    onaddpublic,
  }: {
    repos: Repo[];
    selected: string | null;
    watchedRepos: Set<string>;
    loading: boolean;
    error: string | null;
    onselect: (repo: Repo) => void;
    onaddpublic: (full: string) => Promise<string | null>;
  } = $props();

  let filter = $state("");
  let publicRepo = $state("");
  let addError = $state<string | null>(null);

  const shown = $derived(
    repos.filter((r) => r.full_name.toLowerCase().includes(filter.trim().toLowerCase())),
  );

  async function add() {
    if (!publicRepo.includes("/")) {
      addError = "Usa el formato owner/repo";
      return;
    }
    addError = await onaddpublic(publicRepo.trim());
    if (!addError) publicRepo = "";
  }
</script>

<div class="col">
  <input class="search" placeholder="Buscar repo…" bind:value={filter} />
  <div class="list">
    {#if loading}
      <p class="muted">Cargando repos…</p>
    {:else if error}
      <p class="err">{error}</p>
    {/if}
    {#each shown as repo (repo.full_name)}
      <button class="row" class:sel={repo.full_name === selected} onclick={() => onselect(repo)}>
        <span class="check">{watchedRepos.has(repo.full_name) ? "●" : ""}</span>
        <span class="name">{repo.full_name}</span>
        {#if repo.private}<span class="muted tag">privado</span>{/if}
      </button>
    {/each}
  </div>
  <div class="add">
    <input placeholder="Añadir repo público (owner/repo)" bind:value={publicRepo} onkeydown={(e) => e.key === "Enter" && add()} />
    <button onclick={add}>Añadir</button>
  </div>
  {#if addError}<p class="err">{addError}</p>{/if}
</div>

<style>
  .col {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
  }
  .search,
  .add input {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--input);
    color: inherit;
    font: inherit;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    margin: 8px 0;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
    width: 100%;
    text-align: left;
    padding: 5px 6px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.sel {
    background: var(--selected);
  }
  .check {
    width: 10px;
    color: var(--accent);
    font-size: 10px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    margin-left: auto;
    font-size: 11px;
  }
  .add {
    display: flex;
    gap: 6px;
  }
  .muted {
    color: var(--muted);
  }
  .err {
    color: #d1242f;
    font-size: 12px;
    margin: 4px 0;
  }
</style>
