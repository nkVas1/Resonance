<script lang="ts">
  // Harmonic ring selector — the signature Resonance control.
  // Native panel = the core dot; each profile orbits at a radius
  // proportional to its linear factor. Click a ring to apply.
  import type { ProfileView } from "./ipc";

  let {
    profiles,
    busy,
    onselect,
  }: {
    profiles: ProfileView[];
    busy: boolean;
    onselect: (name: string) => void;
  } = $props();

  const SIZE = 320;
  const C = SIZE / 2;

  // Radius: native dot at 34, outermost ring capped at 140.
  const orbits = $derived.by(() => {
    const sorted = [...profiles].sort((a, b) => a.ratio - b.ratio);
    const maxRatio = Math.max(...sorted.map((p) => p.ratio), 1);
    return sorted.map((p) => ({
      profile: p,
      r: p.ratio <= 1 ? 34 : 34 + ((p.ratio - 1) / Math.max(maxRatio - 1, 0.001)) * 106,
    }));
  });

  let hovered = $state<string | null>(null);
</script>

<svg
  viewBox="0 0 {SIZE} {SIZE}"
  width={SIZE}
  height={SIZE}
  role="radiogroup"
  aria-label="Resolution profiles"
>
  <defs>
    <radialGradient id="coreGlow" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#7c5cff" stop-opacity="0.9" />
      <stop offset="70%" stop-color="#7c5cff" stop-opacity="0.25" />
      <stop offset="100%" stop-color="#7c5cff" stop-opacity="0" />
    </radialGradient>
    <linearGradient id="ringActive" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#7c5cff" />
      <stop offset="100%" stop-color="#4fd8e0" />
    </linearGradient>
  </defs>

  <!-- ambient standing wave -->
  <circle cx={C} cy={C} r="150" class="ambient" />

  {#each orbits as { profile, r } (profile.name)}
    {#if profile.ratio > 1}
      <circle
        cx={C}
        cy={C}
        {r}
        class="orbit"
        class:active={profile.active}
        class:hovered={hovered === profile.name}
        class:unavailable={!profile.available}
        role="radio"
        aria-checked={profile.active}
        aria-label={profile.name}
        tabindex={profile.available && !busy ? 0 : -1}
        onmouseenter={() => (hovered = profile.name)}
        onmouseleave={() => (hovered = null)}
        onclick={() => profile.available && !busy && onselect(profile.name)}
        onkeydown={(e) =>
          e.key === "Enter" && profile.available && !busy && onselect(profile.name)}
      />
      <text
        x={C}
        y={C - r - 7}
        class="orbit-label"
        class:active={profile.active}
        text-anchor="middle"
      >
        {profile.name}
        {#if profile.mode}· {profile.mode.width}×{profile.mode.height}{/if}
      </text>
    {/if}
  {/each}

  <!-- native core -->
  <circle cx={C} cy={C} r="46" fill="url(#coreGlow)" opacity="0.55" />
  {#each orbits as { profile } (profile.name + "-core")}
    {#if profile.ratio <= 1}
      <circle
        cx={C}
        cy={C}
        r="34"
        class="core"
        class:active={profile.active}
        role="radio"
        aria-checked={profile.active}
        aria-label={profile.name}
        tabindex={busy ? -1 : 0}
        onclick={() => !busy && onselect(profile.name)}
        onkeydown={(e) => e.key === "Enter" && !busy && onselect(profile.name)}
      />
      <text x={C} y={C + 4} class="core-label" text-anchor="middle">native</text>
    {/if}
  {/each}
</svg>

<style>
  svg {
    display: block;
    margin: 0 auto;
  }

  .ambient {
    fill: none;
    stroke: var(--line);
    stroke-width: 1;
    stroke-dasharray: 2 7;
    animation: spin 90s linear infinite;
    transform-origin: center;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .orbit {
    fill: none;
    stroke: rgba(139, 147, 167, 0.35);
    stroke-width: 10;
    cursor: pointer;
    transition:
      stroke 260ms var(--ease-out),
      stroke-width 260ms var(--ease-spring),
      filter 260ms var(--ease-out);
    outline: none;
  }

  .orbit.hovered,
  .orbit:focus-visible {
    stroke: rgba(180, 165, 255, 0.75);
    stroke-width: 14;
    filter: drop-shadow(0 0 10px rgba(124, 92, 255, 0.45));
  }

  .orbit.active {
    stroke: url(#ringActive);
    stroke-width: 13;
    filter: drop-shadow(0 0 14px rgba(124, 92, 255, 0.55));
    animation: breathe 4s ease-in-out infinite;
  }

  .orbit.unavailable {
    stroke: rgba(139, 147, 167, 0.12);
    cursor: not-allowed;
  }

  @keyframes breathe {
    0%,
    100% {
      stroke-opacity: 1;
    }
    50% {
      stroke-opacity: 0.7;
    }
  }

  .orbit-label {
    fill: var(--muted);
    font-size: 10.5px;
    letter-spacing: 0.06em;
    pointer-events: none;
    transition: fill 260ms var(--ease-out);
  }

  .orbit-label.active {
    fill: var(--text);
  }

  .core {
    fill: #171b28;
    stroke: rgba(139, 147, 167, 0.4);
    stroke-width: 1.5;
    cursor: pointer;
    transition:
      stroke 260ms var(--ease-out),
      filter 260ms var(--ease-out);
    outline: none;
  }

  .core:hover,
  .core:focus-visible {
    stroke: rgba(180, 165, 255, 0.8);
    filter: drop-shadow(0 0 8px rgba(124, 92, 255, 0.4));
  }

  .core.active {
    stroke: url(#ringActive);
    stroke-width: 2.5;
  }

  .core-label {
    fill: var(--text);
    font-size: 11px;
    letter-spacing: 0.08em;
    pointer-events: none;
  }
</style>
