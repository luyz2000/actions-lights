<script lang="ts">
  import { onMount } from "svelte";
  import { api, onAuthChanged, onOauthFailed, type AuthStatus, type DeviceCode, type TokenSource } from "$lib/api";

  let { auth, onauth }: { auth: AuthStatus | null; onauth: (a: AuthStatus) => void } = $props();

  let token = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let device = $state<DeviceCode | null>(null);
  let copied = $state(false);

  async function run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    busy = true;
    error = null;
    try {
      return await fn();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const recheck = () => run(async () => { await api.refreshNow(); onauth(await api.authStatus()); });
  const saveToken = () => run(async () => { onauth(await api.setToken(token)); token = ""; });
  const clearToken = () => run(async () => onauth(await api.clearToken()));
  const logout = () => run(async () => onauth(await api.oauthLogout()));
  const login = () => run(async () => { device = await api.oauthStart(); });

  async function copyCode() {
    if (!device) return;
    try {
      await navigator.clipboard.writeText(device.user_code);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      /* the code stays visible and selectable */
    }
  }

  onMount(() => {
    const a = onAuthChanged((s) => { device = null; onauth(s); });
    const b = onOauthFailed((msg) => { device = null; error = msg; });
    return () => { a.then((f) => f()); b.then((f) => f()); };
  });

  const inUse = (s: TokenSource) => auth?.source === s;
</script>

<div class="section">
  <header>
    <h1>Cuenta de GitHub</h1>
    <p class="muted">La app usa la primera cuenta que encuentra, en este orden.</p>
  </header>

  {#if auth?.login}
    <div class="current">
      Conectado como <b>@{auth.login}</b>
    </div>
  {:else if auth && auth.source === "none"}
    <div class="current warn">Sin cuenta: solo puedes vigilar repos públicos y se actualiza cada 5 min.</div>
  {/if}

  <ol class="sources">
    <li class:active={inUse("gh")}>
      <div class="head">
        <span class="step">1</span>
        <span class="label">GitHub CLI (<code>gh</code>)</span>
        {#if inUse("gh")}<span class="pill use">En uso</span>{:else if auth?.gh}<span class="pill">Detectada</span>{:else}<span class="pill off">No detectada</span>{/if}
      </div>
      <p class="muted">Usa tu sesión de <code>gh auth login</code>. No necesitas configurar nada más.</p>
      <div class="actions"><button class="btn" disabled={busy} onclick={recheck}>Volver a detectar</button></div>
    </li>

    <li class:active={inUse("manual")}>
      <div class="head">
        <span class="step">2</span>
        <span class="label">Token personal</span>
        {#if inUse("manual")}<span class="pill use">En uso</span>{:else if auth?.manual}<span class="pill">Guardado</span>{/if}
      </div>
      <p class="muted">Token clásico con permiso <code>repo</code>, o fine-grained con <i>Actions: read</i>. Se guarda en el Llavero.</p>
      <div class="actions">
        <input id="token" class="field grow" type="password" placeholder="ghp_… o github_pat_…" bind:value={token} />
        <button class="btn primary" disabled={busy || !token.trim()} onclick={saveToken}>Guardar</button>
        {#if auth?.manual}<button class="btn" disabled={busy} onclick={clearToken}>Borrar</button>{/if}
      </div>
    </li>

    <li class:active={inUse("oauth")}>
      <div class="head">
        <span class="step">3</span>
        <span class="label">Iniciar sesión con GitHub</span>
        {#if inUse("oauth")}<span class="pill use">En uso</span>{:else if auth?.oauth}<span class="pill">Conectada</span>{/if}
      </div>
      {#if !auth?.oauth_available}
        <p class="muted">No disponible en esta versión: falta el client id de la app OAuth al compilar.</p>
      {:else if device}
        <p class="muted">Introduce este código en la página de GitHub que se abrió:</p>
        <div class="code-row">
          <code class="code">{device.user_code}</code>
          <button class="btn" onclick={copyCode}>{copied ? "Copiado" : "Copiar"}</button>
          <button class="btn" onclick={() => api.openUrl(device!.verification_uri)}>Abrir GitHub</button>
        </div>
        <p class="muted small">Esperando autorización…</p>
      {:else}
        <p class="muted">Autoriza la app en el navegador, sin copiar tokens.</p>
        <div class="actions">
          <button class="btn primary" disabled={busy} onclick={login}>Iniciar sesión</button>
          {#if auth?.oauth}<button class="btn" disabled={busy} onclick={logout}>Cerrar sesión</button>{/if}
        </div>
      {/if}
    </li>
  </ol>

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  header p {
    margin: 2px 0 0;
  }
  .current {
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .current.warn {
    background: var(--warn-bg);
    color: var(--warn-fg);
    box-shadow: none;
  }
  .sources {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-radius: 10px;
    box-shadow: var(--shadow);
  }
  .sources li {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .sources li + li {
    border-top: 0.5px solid var(--border);
  }
  .sources li p {
    margin: 0 0 0 30px;
    font-size: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .step {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 600;
    background: var(--surface-2);
    color: var(--muted);
    flex: none;
  }
  .active .step {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .label {
    font-weight: 600;
  }
  .pill {
    margin-left: auto;
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--muted);
  }
  .pill.use {
    background: var(--green);
    color: #fff;
  }
  .pill.off {
    background: transparent;
    border: 0.5px solid var(--border);
  }
  .actions,
  .code-row {
    display: flex;
    gap: 6px;
    margin-left: 30px;
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
    min-width: 160px;
  }
  .code {
    font-family: var(--mono);
    font-size: 18px;
    letter-spacing: 0.12em;
    padding: 2px 10px;
    border-radius: 6px;
    background: var(--surface-2);
    user-select: all;
  }
  code {
    font-family: var(--mono);
    font-size: 0.92em;
  }
  .small {
    font-size: 11px;
  }
  .error {
    margin: 0;
    color: var(--red);
  }
</style>
