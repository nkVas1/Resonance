<script lang="ts">
  // Revert-guard overlay: keep or auto-revert.
  let {
    remaining,
    total,
    onkeep,
    onrevert,
  }: {
    remaining: number;
    total: number;
    onkeep: () => void;
    onrevert: () => void;
  } = $props();

  const R = 42;
  const CIRC = 2 * Math.PI * R;
  const offset = $derived(CIRC * (1 - remaining / Math.max(total, 1)));

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Enter") onkeep();
    if (e.key === "Escape") onrevert();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="veil" role="alertdialog" aria-label="Confirm display change">
  <div class="card">
    <svg viewBox="0 0 100 100" width="120" height="120" aria-hidden="true">
      <circle cx="50" cy="50" r={R} class="track" />
      <circle
        cx="50"
        cy="50"
        r={R}
        class="fuse"
        stroke-dasharray={CIRC}
        stroke-dashoffset={offset}
        transform="rotate(-90 50 50)"
      />
      <text x="50" y="57" text-anchor="middle" class="secs">{remaining}</text>
    </svg>
    <h2>Keep this mode?</h2>
    <p>Reverting automatically if the screen is unreadable.</p>
    <div class="actions">
      <button class="keep" onclick={onkeep}>Keep — Enter</button>
      <button class="revert" onclick={onrevert}>Revert — Esc</button>
    </div>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(6, 8, 13, 0.72);
    backdrop-filter: blur(10px);
    animation: fade 200ms var(--ease-out);
    z-index: 10;
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 28px 32px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    animation: pop 260ms var(--ease-spring);
  }

  @keyframes pop {
    from {
      transform: scale(0.92) translateY(8px);
      opacity: 0;
    }
  }

  .track {
    fill: none;
    stroke: var(--line);
    stroke-width: 5;
  }

  .fuse {
    fill: none;
    stroke: var(--accent);
    stroke-width: 5;
    stroke-linecap: round;
    transition: stroke-dashoffset 1s linear;
  }

  .secs {
    fill: var(--text);
    font-size: 26px;
    font-weight: 600;
  }

  h2 {
    font-size: 16px;
    font-weight: 600;
  }

  p {
    color: var(--muted);
    font-size: 12.5px;
  }

  .actions {
    display: flex;
    gap: 10px;
    margin-top: 14px;
  }

  .actions button {
    padding: 9px 18px;
    border-radius: 10px;
    font-weight: 600;
    font-size: 13px;
    transition:
      transform 160ms var(--ease-spring),
      filter 160ms;
  }

  .actions button:active {
    transform: scale(0.96);
  }

  .keep {
    background: linear-gradient(135deg, var(--accent), #5f8bff);
    color: white;
  }

  .keep:hover {
    filter: brightness(1.12);
  }

  .revert {
    background: var(--bg-raised);
    border: 1px solid var(--line);
    color: var(--muted);
  }

  .revert:hover {
    color: var(--danger);
    border-color: rgba(255, 92, 122, 0.4);
  }
</style>
