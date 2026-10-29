<script>
  // 選關與編隊：最多四人，同一位角色只能上一個版本。開打時 Server 用它記錄的等級組隊、發戰鬥票。
  import { Panel, Button, SectionBar, Tag } from 'bluebolt-ui';
  import { session, startBattle } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, STAGES, ENEMIES, ELEMENT_NAME } from '../data.js';
  import TopBar from './TopBar.svelte';
  import UnitCard from './UnitCard.svelte';

  let { onback, onstart } = $props();

  const TEAM_KEY = 'vl_team';
  function loadTeam() {
    try {
      return JSON.parse(localStorage.getItem(TEAM_KEY) ?? '[]');
    } catch {
      return [];
    }
  }

  const ownedIds = $derived(new Set(session.profile.costumes.map((c) => c.id)));
  /** 上次的隊伍（拿掉已經沒有的），沒有就用前四個 */
  function initialTeam() {
    const saved = loadTeam().filter((id) => session.profile.costumes.some((c) => c.id === id));
    return saved.length ? saved : session.profile.costumes.slice(0, 4).map((c) => c.id);
  }
  let team = $state(initialTeam());

  const clears = $derived(session.profile.clears);
  const unlocked = (i) => i === 0 || clears[STAGES[i - 1].id] !== undefined;
  let stage = $state(STAGES.find((s, i) => unlocked(i) && clears[s.id] === undefined)?.id ?? STAGES[0].id);
  const current = $derived(STAGES.find((s) => s.id === stage));
  const enemies = $derived([...new Set(current.waves.flat())].map((id) => ENEMIES.get(id)));

  const levelOf = (id) => session.profile.costumes.find((c) => c.id === id)?.level;

  function toggle(id) {
    if (team.includes(id)) {
      team = team.filter((x) => x !== id);
      return;
    }
    const ch = COSTUMES.get(id).character;
    // 同一位角色換成這個版本
    const rest = team.filter((x) => COSTUMES.get(x).character !== ch);
    if (rest.length >= 4) return;
    team = [...rest, id];
  }

  async function go() {
    try {
      localStorage.setItem(TEAM_KEY, JSON.stringify(team));
    } catch {
      /* 記不住隊伍也沒關係 */
    }
    try {
      onstart(await startBattle(stage, team));
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }
</script>

<TopBar title={t('出擊')} en="Sortie" num="01" {onback} />

<section class="stages">
  <SectionBar title={t('第一章　新傳說的入口')} en="Chapter 1" />
  <ol>
    {#each STAGES as s, i (s.id)}
      <li style:--i={i}>
        <button type="button" class:on={s.id === stage} disabled={!unlocked(i)} onclick={() => (stage = s.id)}>
          <span class="code">{s.id}</span>
          <span class="name">{t(s.name)}</span>
          {#if s.env}<span class="env">{t(s.env.name)}</span>{/if}
          {#if clears[s.id] !== undefined}<span class="clear">CLEAR</span>{:else if unlocked(i)}<span class="first">✦{s.rewards.first_stardust}</span>{:else}<span class="lock">LOCK</span>{/if}
        </button>
      </li>
    {/each}
  </ol>
</section>

<section class="side">
  <Panel num={current.id} title={t(current.name)} sub={`${current.waves.length} Waves`} tone="glass" from="right">
    <div class="foes">
      {#each enemies as e (e.id)}
        <span class="foe el-{e.element}"><span class="el-mark">{t(ELEMENT_NAME[e.element])}</span>{t(e.name)}{#if e.boss}<Tag variant="yellow" size="sm">BOSS</Tag>{/if}</span>
      {/each}
    </div>
    {#if current.env}<p class="envtext"><Tag size="sm">{t(current.env.name)}</Tag> {t(current.env.text)}</p>{/if}
    <p class="meta">
      {t('獎勵：金幣 {g}・練習筆記 {n}', { g: current.rewards.gold, n: current.rewards.notes })}
      ・{t('★ {n} 回合內', { n: current.star_turns })}
    </p>
  </Panel>

  <SectionBar title={t('隊伍')} en={`Party ${team.length}/4`} />
  <div class="slots">
    {#each [0, 1, 2, 3] as i (i)}
      {#if team[i] && ownedIds.has(team[i])}
        <UnitCard costume={team[i]} level={levelOf(team[i])} num={i + 1} width={110} onclick={() => toggle(team[i])} />
      {:else}
        <div class="empty">{i + 1}</div>
      {/if}
    {/each}
    <div class="go">
      <Button size="lg" en="Go" disabled={team.length === 0 || session.busy} onclick={go}>{t('出擊！')}</Button>
    </div>
  </div>

  <SectionBar title={t('點角色加入或移出隊伍')} en="Members" />
  <div class="pool">
    {#each session.profile.costumes as c (c.id)}
      <UnitCard costume={c.id} level={c.level} selected={team.includes(c.id)} width={86} onclick={() => toggle(c.id)} />
    {/each}
  </div>
</section>

<style>
  .stages {
    position: absolute;
    left: 40px;
    top: 112px;
    width: 420px;
    bottom: 28px;
  }

  ol {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    padding-left: calc(var(--i) * 4px);
    animation: in var(--bb-dur-4) var(--bb-ease) both;
    animation-delay: calc(var(--i) * var(--bb-stagger));
  }

  @keyframes in {
    from {
      transform: translateX(calc(-40px * var(--bb-motion)));
      opacity: 0;
    }
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border: 0;
    background: var(--bb-glass-strong);
    box-shadow: inset 0 0 0 1px var(--bb-line);
    clip-path: polygon(12px 0, 100% 0, calc(100% - 12px) 100%, 0 100%);
    text-align: left;
    transition: transform var(--bb-dur-2) var(--bb-ease);
  }

  li button:not(:disabled):hover {
    transform: translateX(6px);
  }

  li button.on {
    background: var(--bb-strip);
    color: var(--bb-on-strip);
  }

  li button:disabled {
    opacity: 0.45;
  }

  .code {
    font: italic 400 22px var(--bb-font-display);
    width: 46px;
    color: var(--bb-blue-text);
  }

  .on .code {
    color: var(--bb-yellow);
  }

  .name {
    flex: 1;
    font-weight: 800;
  }

  .env {
    font-size: 11px;
    padding: 1px 8px;
    background: var(--bb-blue-soft);
    color: var(--bb-blue-text);
  }

  .clear,
  .first,
  .lock {
    font: 700 12px var(--bb-font-cond);
    letter-spacing: 0.12em;
  }

  .clear {
    color: var(--gale);
  }

  .first {
    color: var(--bb-blue-2);
  }

  .side {
    position: absolute;
    left: 490px;
    right: 40px;
    top: 112px;
    bottom: 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .foes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
  }

  .foe {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 700;
    font-size: 14px;
  }

  .foe .el-mark {
    width: 20px;
    height: 20px;
    font-size: 10px;
  }

  .envtext,
  .meta {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--bb-ink-2);
  }

  .slots {
    display: flex;
    gap: 12px;
    align-items: flex-end;
  }

  .empty {
    width: 110px;
    height: 152px;
    display: grid;
    place-items: center;
    border: 2px dashed var(--bb-line-2);
    font: italic 400 40px var(--bb-font-display);
    color: var(--bb-ink-4);
  }

  .go {
    margin-left: auto;
    align-self: center;
  }

  .pool {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    overflow-y: auto;
    padding: 4px;
    flex: 1;
    align-content: flex-start;
  }
</style>
