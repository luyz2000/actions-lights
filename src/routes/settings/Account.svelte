<script lang="ts">
  import { api, type AuthStatus } from "$lib/api";

  let { auth, onauth }: { auth: AuthStatus | null; onauth: (a: AuthStatus) => void } = $props();

  let token = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  const SOURCES = { gh: "gh CLI", manual: "token manual", none: "sin token" } as const;

  async function save() {
    busy = true;
    error = null;
    try {
      onauth(await api.setToken(token));
      token = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function clear() {
    busy = true;
    try {
      onauth(await api.clearToken());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="account">
  <div>
    <b>Cuenta:</b>
    {auth ? SOURCES[auth.source] : "…"}{#if auth?.login} · <span class="login">@{auth.login}</span>{/if}
  </div>
  <div class="row">
    <input type="password" placeholder="Pegar token (repo, o fine-grained con Actions: read)" bind:value={token} />
    <button disabled={busy || !token.trim()} onclick={save}>Guardar token</button>
    {#if auth?.source === "manual"}
      <button disabled={busy} onclick={clear}>Borrar token manual</button>
    {/if}
  </div>
  {#if error}<div class="err">{error}</div>{/if}
</div>

<style>
  .account {
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--input);
    color: inherit;
    font: inherit;
  }
  .login {
    color: var(--accent);
  }
  .err {
    color: #d1242f;
    margin-top: 4px;
  }
</style>
