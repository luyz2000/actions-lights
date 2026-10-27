<script lang="ts">
  import { api, type Card } from "$lib/api";
  import { formatAgo, formatDuration } from "$lib/format";

  let { card, now }: { card: Card; now: Date } = $props();

  const running = $derived(card.light === "yellow");
  const time = $derived.by(() => {
    if (running && card.started_at) {
      const elapsed = (now.getTime() - new Date(card.started_at).getTime()) / 1000;
      const expected = card.expected_secs ? ` / ~${formatDuration(card.expected_secs)}` : "";
      return formatDuration(elapsed) + expected;
    }
    if (card.completed_at) {
      return card.light === "red" ? `falló ${formatAgo(card.completed_at, now)}` : formatAgo(card.completed_at, now);
    }
    return "";
  });
  const sub = $derived(
    card.error ??
      (card.failed_step ? `Falló en: ${card.failed_step}` : null) ??
      card.label ??
      (running ? card.step_label : null),
  );

  function open() {
    if (card.run_url) api.openUrl(card.run_url);
  }
</script>

<button class="card" class:clickable={!!card.run_url} onclick={open} title={card.run_url ?? ""}>
  <div class="top">
    <span class="dot {card.light}"></span>
    <span class="name">{card.workflow_name} · {card.branch ?? "cualquiera"}</span>
    <small>{time}</small>
  </div>
  {#if running && card.progress !== null}
    <div class="pb"><div style="width: {Math.round(card.progress * 100)}%"></div></div>
  {/if}
  {#if sub}
    <div class="sub" class:err={!!card.error}>{sub}</div>
  {/if}
</button>

<style>
  .card {
    display: block;
    width: 100%;
    text-align: left;
    background: var(--card);
    color: inherit;
    border: 0;
    border-radius: 8px;
    padding: 8px 10px;
    margin-bottom: 8px;
    font: inherit;
    cursor: default;
  }
  .card.clickable {
    cursor: pointer;
  }
  .card.clickable:hover {
    background: var(--card-hover);
  }
  .top {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .top small {
    color: var(--muted);
    margin-left: auto;
    white-space: nowrap;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }
  .gray { background: #8b949e; }
  .yellow { background: #d29922; }
  .green { background: #2ea043; }
  .red { background: #f85149; }
  .pb {
    height: 6px;
    background: var(--track);
    border-radius: 3px;
    margin-top: 7px;
    overflow: hidden;
  }
  .pb > div {
    height: 100%;
    background: #d29922;
    border-radius: 3px;
    transition: width 0.6s ease;
  }
  .sub {
    color: var(--muted);
    font-size: 11px;
    margin-top: 4px;
  }
  .sub.err {
    color: #f85149;
  }
</style>
