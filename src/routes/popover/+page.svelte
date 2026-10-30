<script lang="ts">
  import { onMount } from "svelte";
  import "$lib/theme.css";
  import { api, onDashboard, type DashboardState, type Light } from "$lib/api";
  import { formatAgo } from "$lib/format";
  import StatusIcon from "$lib/StatusIcon.svelte";
  import Card from "./Card.svelte";

  let dash = $state<DashboardState | null>(null);
  let now = $state(new Date());
  let refreshing = $state(false);

  const counts = $derived.by(() => {
    const c: Record<Light, number> = { red: 0, yellow: 0, green: 0, gray: 0 };
    for (const g of dash?.groups ?? []) for (const card of g.cards) c[card.light]++;
    return c;
  });
  const overall = $derived<Light>(
    counts.red ? "red" : counts.yellow ? "yellow" : counts.green ? "green" : "gray",
  );
  const headline = $derived.by(() => {
    if (!dash?.last_updated) return "Consultando GitHub…";
    if (counts.red) return counts.red === 1 ? "1 workflow falló" : `${counts.red} workflows fallaron`;
    if (counts.yellow) return counts.yellow === 1 ? "1 workflow en curso" : `${counts.yellow} workflows en curso`;
    if (counts.green) return "Todo en verde";
    return "Sin actividad";
  });
  const detail = $derived(
    [
      counts.yellow && counts.red ? `${counts.yellow} en curso` : null,
      counts.green && (counts.red || counts.yellow) ? `${counts.green} OK` : null,
    ]
      .filter(Boolean)
      .join(" · "),
  );

  async function refresh() {
    refreshing = true;
    await api.refreshNow().catch(() => {});
    setTimeout(() => (refreshing = false), 800);
  }

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
  <header>
    <StatusIcon light={overall} size={20} />
    <div class="summary">
      <div class="headline">{headline}</div>
      {#if detail}<div class="detail">{detail}</div>{/if}
    </div>
    <button class="icon-btn" class:spinning={refreshing} onclick={refresh} title="Actualizar ahora" aria-label="Actualizar ahora">
      <svg width="15" height="15" viewBox="0 0 16 16"><path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
    </button>
    <button class="icon-btn" onclick={() => api.openSettings()} title="Ajustes" aria-label="Ajustes">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /><circle cx="12" cy="12" r="3" /></svg>
    </button>
  </header>

  {#if dash?.banner}
    {#if dash.banner_url}
      <button class="banner link" onclick={() => api.openUrl(dash!.banner_url!)}>{dash.banner}</button>
    {:else}
      <div class="banner">{dash.banner}</div>
    {/if}
  {/if}

  <div class="list">
    {#if dash && dash.groups.length === 0}
      <div class="empty">
        <p class="title">Aún no vigilas ningún workflow</p>
        <p class="muted">Elige los repos y workflows de deploy que quieres seguir desde la barra.</p>
        <button class="btn primary" onclick={() => api.openSettings()}>Elegir workflows</button>
      </div>
    {:else if dash}
      {#each dash.groups as group (group.repo)}
        <section>
          <h2>{group.repo}</h2>
          {#each group.cards as card (card.key)}
            <Card {card} {now} />
          {/each}
        </section>
      {/each}
    {/if}
  </div>

  <footer>
    <span class="muted">
      {dash?.last_updated ? `Actualizado ${formatAgo(dash.last_updated, now)}` : "Conectando…"}
      {#if dash?.auth.login} · @{dash.auth.login}{/if}
    </span>
    <button class="text-btn" onclick={() => api.quit()}>Salir</button>
  </footer>
</main>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
    height: 100%;
  }
  main {
    box-sizing: border-box;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-radius: 12px;
    border: 0.5px solid var(--border);
    user-select: none;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 12px 10px 14px;
    border-bottom: 0.5px solid var(--border);
  }
  .summary {
    flex: 1;
    min-width: 0;
  }
  .headline {
    font-weight: 600;
    font-size: 14px;
  }
  .detail {
    color: var(--muted);
    font-size: 12px;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--hover);
    color: var(--fg);
  }
  .icon-btn.spinning svg {
    animation: spin 0.8s linear;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .banner {
    margin: 8px 10px 0;
    background: var(--warn-bg);
    color: var(--warn-fg);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 12px;
    word-break: break-word;
    user-select: text;
  }
  .banner.link {
    display: block;
    width: calc(100% - 20px);
    text-align: left;
    border: 0;
    cursor: pointer;
    text-decoration: underline;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 6px 8px;
  }
  section + section {
    border-top: 0.5px solid var(--border);
    margin-top: 4px;
  }
  h2 {
    margin: 0;
    padding: 10px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
    padding: 70px 24px 0;
  }
  .empty p {
    margin: 0;
  }
  .empty .title {
    font-weight: 600;
  }
  .empty .btn {
    margin-top: 8px;
  }
  footer {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-top: 0.5px solid var(--border);
    font-size: 12px;
  }
  .text-btn {
    margin-left: auto;
    border: 0;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    border-radius: 5px;
    padding: 2px 6px;
  }
  .text-btn:hover {
    background: var(--hover);
    color: var(--fg);
  }
</style>
