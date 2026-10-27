<script lang="ts">
  import { onMount } from "svelte";
  import { api, onDashboard, type DashboardState } from "$lib/api";
  import Card from "./Card.svelte";

  let dash = $state<DashboardState | null>(null);
  let now = $state(new Date());

  onMount(() => {
    api.getDashboard().then((d) => (dash = d));
    const unlisten = onDashboard((d) => (dash = d));
    const tick = setInterval(() => (now = new Date()), 1000);
    return () => {
      clearInterval(tick);
      unlisten.then((f) => f());
    };
  });
</script>

<main>
  {#if dash?.banner}
    <div class="banner">{dash.banner}</div>
  {/if}

  <div class="list">
    {#if !dash || dash.groups.length === 0}
      <div class="empty">
        <p>No vigilas ningún workflow todavía</p>
        <button class="primary" onclick={() => api.openSettings()}>Elegir repos y workflows</button>
      </div>
    {:else}
      {#each dash.groups as group (group.repo)}
        <div class="h">{group.repo}</div>
        {#each group.cards as card (card.key)}
          <Card {card} {now} />
        {/each}
      {/each}
    {/if}
  </div>

  <footer>
    <button onclick={() => api.refreshNow()}>Actualizar ahora</button>
    <button onclick={() => api.openSettings()}>Repos y workflows…</button>
    <button onclick={() => api.quit()}>Salir</button>
  </footer>
</main>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
  }
  main {
    --bg: #2b2b31;
    --card: #34343b;
    --card-hover: #3d3d45;
    --track: #45454d;
    --muted: #9aa0a6;
    --fg: #e6e6e6;
    box-sizing: border-box;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    font: 13px -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, sans-serif;
    border-radius: 12px;
    padding: 10px;
    user-select: none;
  }
  .banner {
    background: #5a3d0f;
    color: #ffd38a;
    border-radius: 8px;
    padding: 8px 10px;
    margin-bottom: 8px;
    font-size: 12px;
    word-break: break-word;
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .h {
    color: var(--muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin: 6px 2px;
  }
  .empty {
    text-align: center;
    color: var(--muted);
    padding-top: 80px;
  }
  footer {
    display: flex;
    gap: 6px;
    border-top: 1px solid #3a3a42;
    padding-top: 8px;
  }
  footer button,
  .primary {
    background: transparent;
    color: var(--fg);
    border: 0;
    border-radius: 6px;
    padding: 5px 8px;
    font: inherit;
    cursor: pointer;
  }
  footer button:hover {
    background: var(--card);
  }
  footer button:last-child {
    margin-left: auto;
    color: var(--muted);
  }
  .primary {
    background: #3a5ccc;
  }
</style>
