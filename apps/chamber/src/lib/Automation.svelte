<script lang="ts">
  import type { Snapshot, NewRule, TriggerKind } from "./ipc";

  let {
    snap,
    busy,
    ontoggle,
    onaddrule,
    onremoverule,
    onautostart,
  }: {
    snap: Snapshot;
    busy: boolean;
    ontoggle: (enabled: boolean) => void;
    onaddrule: (rule: NewRule) => void;
    onremoverule: (name: string) => void;
    onautostart: (enabled: boolean) => void;
  } = $props();

  let adding = $state(false);
  let kind = $state<TriggerKind>("foreground");
  let value = $state("");
  let ruleName = $state("");
  let profile = $state("");
  let priority = $state(0);

  function submit(e: Event) {
    e.preventDefault();
    const name = ruleName.trim() || defaultName();
    const prof = profile || snap.profiles[1]?.name || snap.profiles[0]!.name;
    onaddrule({ name, kind, value: value.trim(), profile: prof, priority });
    adding = false;
    value = "";
    ruleName = "";
  }

  function defaultName(): string {
    if (kind === "power") return value === "battery" ? "On battery" : "On AC";
    return value.trim().replace(/\.exe$/i, "") || "rule";
  }

  const valuePlaceholder = $derived(
    kind === "power" ? "" : kind === "foreground" ? "photoshop.exe" : "obs64.exe",
  );
</script>

<div class="auto">
  <div class="master">
    <div>
      <h3>Automation</h3>
      <p class="sub">
        {#if snap.automationEnabled}
          Rules drive your resolution automatically
        {:else}
          Switch profiles by app, power &amp; more
        {/if}
      </p>
    </div>
    <button
      class="switch"
      class:on={snap.automationEnabled}
      role="switch"
      aria-label="Toggle automation"
      aria-checked={snap.automationEnabled}
      disabled={busy}
      onclick={() => ontoggle(!snap.automationEnabled)}
    >
      <span class="dot"></span>
    </button>
  </div>

  {#if snap.activeCause}
    <div class="why">
      <span class="why-dot"></span>
      active&nbsp;·&nbsp;<strong>{snap.activeCause}</strong>
    </div>
  {/if}

  <div class="rules">
    {#if snap.rules.length === 0}
      <p class="empty">No rules yet. Add one to let Resonance react to what you're doing.</p>
    {:else}
      {#each snap.rules as rule (rule.name)}
        <div class="rule" class:active={rule.active}>
          <div class="rule-main">
            <span class="rule-name">{rule.name}</span>
            <span class="rule-trigger">{rule.trigger} → <em>{rule.profile}</em></span>
          </div>
          {#if rule.active}<span class="live">live</span>{/if}
          <button
            class="del"
            aria-label="Remove rule {rule.name}"
            disabled={busy}
            onclick={() => onremoverule(rule.name)}>✕</button
          >
        </div>
      {/each}
    {/if}
  </div>

  {#if adding}
    <form class="add-form" onsubmit={submit}>
      <div class="field">
        <label for="kind">When</label>
        <select id="kind" bind:value={kind}>
          <option value="foreground">app is focused</option>
          <option value="running">app is running</option>
          <option value="power">power source is</option>
        </select>
      </div>
      <div class="field">
        <label for="val">{kind === "power" ? "Source" : "App (.exe)"}</label>
        {#if kind === "power"}
          <select id="val" bind:value>
            <option value="battery">on battery</option>
            <option value="ac">on AC</option>
          </select>
        {:else}
          <input id="val" bind:value placeholder={valuePlaceholder} autocomplete="off" />
        {/if}
      </div>
      <div class="field">
        <label for="prof">Use profile</label>
        <select id="prof" bind:value={profile}>
          {#each snap.profiles as p (p.name)}
            <option value={p.name}>{p.name}</option>
          {/each}
        </select>
      </div>
      <div class="form-actions">
        <button type="submit" class="save" disabled={busy || (kind !== "power" && !value.trim())}>
          Add rule
        </button>
        <button type="button" class="cancel" onclick={() => (adding = false)}>Cancel</button>
      </div>
    </form>
  {:else}
    <button class="add-btn" disabled={busy} onclick={() => (adding = true)}>+ New rule</button>
  {/if}

  <div class="startup">
    <div>
      <span class="startup-title">Start with Windows</span>
      <p class="sub">Launch to the tray automatically at login</p>
    </div>
    <button
      class="switch"
      class:on={snap.autostartEnabled}
      role="switch"
      aria-label="Start with Windows"
      aria-checked={snap.autostartEnabled}
      disabled={busy}
      onclick={() => onautostart(!snap.autostartEnabled)}
    >
      <span class="dot"></span>
    </button>
  </div>
</div>

<style>
  .auto {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 4px 2px;
  }

  .master {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .master h3 {
    font-size: 1.05rem;
    font-weight: 640;
    letter-spacing: -0.01em;
  }

  .sub {
    color: var(--muted);
    font-size: 0.82rem;
    margin-top: 2px;
  }

  .switch {
    flex-shrink: 0;
    width: 46px;
    height: 26px;
    border-radius: 999px;
    background: var(--bg-raised);
    border: 1px solid var(--line);
    position: relative;
    transition: background 0.25s var(--ease-out);
  }

  .switch .dot {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--muted);
    transition:
      transform 0.28s var(--ease-spring),
      background 0.25s;
  }

  .switch.on {
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    border-color: transparent;
  }

  .switch.on .dot {
    transform: translateX(20px);
    background: #fff;
  }

  .why {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.82rem;
    color: var(--muted);
    padding: 8px 11px;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
    border-radius: 9px;
  }

  .why strong {
    color: var(--text);
    font-weight: 550;
  }

  .why-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-2);
    box-shadow: 0 0 8px var(--accent-2);
  }

  .rules {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .empty {
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.45;
    padding: 6px 2px;
  }

  .rule {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px 11px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 10px;
    transition: border-color 0.2s;
  }

  .rule.active {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .rule-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .rule-name {
    font-size: 0.88rem;
    font-weight: 550;
  }

  .rule-trigger {
    font-size: 0.76rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rule-trigger em {
    color: var(--accent-2);
    font-style: normal;
  }

  .live {
    font-size: 0.62rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent-2);
    padding: 2px 6px;
    border-radius: 5px;
    background: color-mix(in srgb, var(--accent-2) 12%, transparent);
  }

  .del {
    color: var(--muted);
    font-size: 0.85rem;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    transition:
      color 0.2s,
      background 0.2s;
  }

  .del:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }

  .add-btn {
    align-self: flex-start;
    font-size: 0.84rem;
    color: var(--accent-2);
    padding: 7px 13px;
    border-radius: 9px;
    border: 1px dashed color-mix(in srgb, var(--accent-2) 35%, transparent);
    transition:
      background 0.2s,
      border-color 0.2s;
  }

  .add-btn:hover {
    background: color-mix(in srgb, var(--accent-2) 8%, transparent);
  }

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 13px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 11px;
    animation: slidein 0.25s var(--ease-out);
  }

  @keyframes slidein {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field label {
    font-size: 0.72rem;
    color: var(--muted);
    letter-spacing: 0.03em;
  }

  select,
  input {
    font: inherit;
    font-size: 0.86rem;
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 8px 10px;
    outline: none;
    transition: border-color 0.2s;
  }

  select:focus,
  input:focus {
    border-color: var(--accent);
  }

  .form-actions {
    display: flex;
    gap: 8px;
    margin-top: 3px;
  }

  .save {
    flex: 1;
    font-weight: 600;
    font-size: 0.85rem;
    color: #fff;
    padding: 9px;
    border-radius: 9px;
    background: linear-gradient(135deg, var(--accent), #5f8bff);
    transition:
      filter 0.2s,
      opacity 0.2s;
  }

  .save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save:not(:disabled):hover {
    filter: brightness(1.1);
  }

  .cancel {
    font-size: 0.85rem;
    color: var(--muted);
    padding: 9px 14px;
    border-radius: 9px;
    border: 1px solid var(--line);
  }

  .cancel:hover {
    color: var(--text);
  }

  .startup {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 4px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }

  .startup-title {
    font-size: 0.9rem;
    font-weight: 550;
  }
</style>
