<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, type Snapshot } from "./lib/ipc";
  import Rings from "./lib/Rings.svelte";
  import Countdown from "./lib/Countdown.svelte";

  let snap = $state<Snapshot | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let countdown = $state<number | null>(null);

  const superRes = $derived(snap?.superRes ?? false);

  async function refresh() {
    try {
      snap = await ipc.snapshot();
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function run(op: () => Promise<Snapshot>) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      snap = await op();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    refresh();
    const un1 = ipc.onSnapshot((s) => {
      snap = s;
      if (!s.guardPending) countdown = null;
    });
    const un2 = ipc.onRevertTick((t) => (countdown = t.remaining));
    return () => {
      un1.then((f) => f());
      un2.then((f) => f());
    };
  });
</script>

<main>
  <header>
    <div class="mark" aria-hidden="true">
      <span></span><span></span><span></span>
    </div>
    <h1>Resonance</h1>
    {#if superRes}
      <span class="badge">super-resolution</span>
    {/if}
  </header>

  {#if snap}
    <Rings
      profiles={snap.profiles}
      {busy}
      onselect={(name) => run(() => ipc.applyProfile(name))}
    />

    <section class="state" aria-live="polite">
      <div class="row">
        <span class="k">rendering</span>
        <span class="v">{snap.mode.width}×{snap.mode.height} @{snap.mode.hz}Hz</span>
      </div>
      <div class="row">
        <span class="k">panel</span>
        <span class="v">{snap.native[0]}×{snap.native[1]} · scale {snap.scale}%</span>
      </div>
      <div class="row">
        <span class="k">gpu</span>
        <span class="v dim">{snap.adapter}</span>
      </div>
    </section>
  {:else}
    <p class="loading">tuning…</p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if countdown !== null && snap?.guardPending}
    <Countdown
      remaining={countdown}
      total={snap.confirmTimeout}
      onkeep={() => run(() => ipc.confirm())}
      onrevert={() => run(() => ipc.revert())}
    />
  {/if}
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 18px 20px 20px;
    gap: 8px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .mark {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 18px;
  }

  .mark span {
    width: 3px;
    border-radius: 2px;
    background: linear-gradient(180deg, var(--accent), var(--accent-2));
    animation: wave 1.8s var(--ease-out) infinite;
  }

  .mark span:nth-child(1) {
    height: 10px;
  }
  .mark span:nth-child(2) {
    height: 18px;
    animation-delay: 0.15s;
  }
  .mark span:nth-child(3) {
    height: 13px;
    animation-delay: 0.3s;
  }

  @keyframes wave {
    0%,
    100% {
      transform: scaleY(1);
    }
    50% {
      transform: scaleY(0.55);
    }
  }

  h1 {
    font-size: 16px;
    font-weight: 650;
    letter-spacing: 0.02em;
  }

  .badge {
    margin-left: auto;
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 10.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent-2);
    border: 1px solid rgba(79, 216, 224, 0.35);
    background: rgba(79, 216, 224, 0.08);
    animation: badgein 320ms var(--ease-spring);
  }

  @keyframes badgein {
    from {
      transform: scale(0.8);
      opacity: 0;
    }
  }

  .state {
    margin-top: auto;
    padding: 14px 16px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }

  .k {
    color: var(--muted);
    letter-spacing: 0.04em;
  }

  .v {
    font-variant-numeric: tabular-nums;
    font-weight: 550;
  }

  .v.dim {
    color: var(--muted);
    font-weight: 400;
  }

  .loading {
    margin: auto;
    color: var(--muted);
    letter-spacing: 0.2em;
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }

  .error {
    padding: 10px 14px;
    border-radius: 10px;
    background: rgba(255, 92, 122, 0.1);
    border: 1px solid rgba(255, 92, 122, 0.3);
    color: var(--danger);
    font-size: 12.5px;
  }
</style>
