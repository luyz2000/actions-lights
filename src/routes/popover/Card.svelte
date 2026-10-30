<script lang="ts">
  import { api, type Card } from "$lib/api";
  import { formatAgo, formatDuration } from "$lib/format";
  import StatusIcon from "$lib/StatusIcon.svelte";

  let { card, now }: { card: Card; now: Date } = $props();

  const running = $derived(card.light === "yellow");
  const time = $derived.by(() => {
    if (running && card.started_at) {
      const elapsed = (now.getTime() - new Date(card.started_at).getTime()) / 1000;
      return formatDuration(elapsed);
    }
    return card.completed_at ? formatAgo(card.completed_at, now) : "";
  });
  const sub = $derived(
    card.error ??
      (card.failed_step ? `Falló en ${card.failed_step}` : null) ??
      card.label ??
      (running ? card.step_label : null),
  );
  const pct = $derived(card.progress !== null ? Math.round(card.progress * 100) : null);

  function open() {
    if (card.run_url) api.openUrl(card.run_url);
  }
</script>

<button class="row {card.light}" disabled={!card.run_url} onclick={open} title={card.run_url ? "Abrir el run en GitHub" : ""}>
  <StatusIcon light={card.light} />
  <div class="body">
    <div class="line">
      <span class="name">{card.workflow_name}</span>
      <span class="time">{time}</span>
    </div>
    <div><span class="branch">{card.branch ?? "cualquier rama"}</span></div>
    {#if running && pct !== null}
      <div class="progress" role="progressbar" aria-valuenow={pct} aria-valuemin="0" aria-valuemax="100">
        <div style="width: {pct}%"></div>
      </div>
    {/if}
    {#if sub || (running && card.expected_secs)}
      <div class="sub" class:err={!!card.error || !!card.failed_step}>
        <span class="text">{sub ?? ""}</span>
        {#if running && card.expected_secs}<span class="eta">~{formatDuration(card.expected_secs)} en total</span>{/if}
      </div>
    {/if}
  </div>
</button>

<style>
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 8px;
    padding: 7px 8px;
    cursor: pointer;
  }
  .row:disabled {
    cursor: default;
  }
  .row:hover:not(:disabled) {
    background: var(--hover);
  }
  .row :global(.icon) {
    margin-top: 1px;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .line {
    display: flex;
    gap: 6px;
    align-items: baseline;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .time {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .progress {
    height: 4px;
    background: var(--track);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress > div {
    height: 100%;
    background: var(--yellow);
    border-radius: 2px;
    transition: width 0.6s ease;
  }
  .sub {
    display: flex;
    gap: 8px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .sub .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub .eta {
    margin-left: auto;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .sub.err .text {
    color: var(--red);
  }
</style>
