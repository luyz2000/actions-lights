<script lang="ts">
  import type { Light } from "./api";

  let { light, size = 16 }: { light: Light; size?: number } = $props();
</script>

<svg class="icon {light}" width={size} height={size} viewBox="0 0 16 16" aria-label={light}>
  {#if light === "yellow"}
    <circle cx="8" cy="8" r="6" fill="none" stroke="var(--track)" stroke-width="2.2" />
    <circle class="spin" cx="8" cy="8" r="6" fill="none" stroke="var(--yellow)" stroke-width="2.2" stroke-linecap="round" stroke-dasharray="12 26" />
  {:else if light === "green"}
    <circle cx="8" cy="8" r="7.5" fill="var(--green)" />
    <path d="M4.8 8.2l2.1 2.1 4.3-4.5" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
  {:else if light === "red"}
    <circle cx="8" cy="8" r="7.5" fill="var(--red)" />
    <path d="M5.5 5.5l5 5m0-5l-5 5" stroke="#fff" stroke-width="1.8" stroke-linecap="round" />
  {:else}
    <circle cx="8" cy="8" r="7" fill="none" stroke="var(--gray)" stroke-width="1.6" />
    <path d="M5.5 8h5" stroke="var(--gray)" stroke-width="1.6" stroke-linecap="round" />
  {/if}
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
  .spin {
    transform-origin: 8px 8px;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
