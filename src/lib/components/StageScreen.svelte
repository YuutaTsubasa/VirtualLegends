<script>
  // 選關與編隊：最多四人，同一位角色只能上一個版本。開打時 Server 用它記錄的等級組隊、發戰鬥票。
  import { session, startBattle } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, STAGES, ENEMIES } from '../data.js';
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
  const enemyNames = $derived([...new Set(current.waves.flat())].map((id) => t(ENEMIES.get(id).name)));

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

<TopBar title={t('出擊')} {onback} />

<section class="stages">
  <h3>{t('第一章　雀館的怪聲')}</h3>
  {#each STAGES as s, i (s.id)}
    <button type="button" class="stage-item" class:on={s.id === stage} disabled={!unlocked(i)} onclick={() => (stage = s.id)}>
      <span class="id">{s.id}</span>
      <span class="name">{t(s.name)}</span>
      {#if clears[s.id] !== undefined}<span class="clear">{t('通關')}</span>{:else if unlocked(i)}<span class="first">{t('首通 ✦{n}', { n: s.rewards.first_stardust })}</span>{/if}
    </button>
  {/each}
</section>

<section class="team">
  <div class="head">
    <div>
      <b>{current.id} {t(current.name)}</b>
      <small>{t('{n} 波・敵人：{list}', { n: current.waves.length, list: enemyNames.join('、') })}</small>
      <small>{t('獎勵：金幣 {g}・練習筆記 {n}', { g: current.rewards.gold, n: current.rewards.notes })}</small>
    </div>
    <button class="btn" type="button" disabled={team.length === 0 || session.busy} onclick={go}>{t('出擊！')}</button>
  </div>
  <p class="label">{t('隊伍（{n}／4）', { n: team.length })}</p>
  <div class="slots">
    {#each [0, 1, 2, 3] as i (i)}
      {#if team[i] && ownedIds.has(team[i])}
        <UnitCard costume={team[i]} level={levelOf(team[i])} onclick={() => toggle(team[i])} />
      {:else}
        <div class="empty">{t('空位')}</div>
      {/if}
    {/each}
  </div>
  <p class="label">{t('點角色加入或移出隊伍')}</p>
  <div class="pool">
    {#each session.profile.costumes as c (c.id)}
      <UnitCard costume={c.id} level={c.level} selected={team.includes(c.id)} onclick={() => toggle(c.id)} />
    {/each}
  </div>
</section>

<style>
  .stages {
    position: absolute;
    top: 10cqh;
    left: 3cqh;
    width: 46cqh;
    bottom: 3cqh;
    display: flex;
    flex-direction: column;
    gap: 1.2cqh;
  }

  .stages h3 {
    margin: 0 0 1cqh;
    color: var(--gold);
  }

  .stage-item {
    display: flex;
    align-items: center;
    gap: 1.6cqh;
    padding: 1.4cqh 2cqh;
    border: var(--line) solid var(--gold-deep);
    border-radius: 1cqh;
    background: rgba(0, 0, 0, 0.35);
    text-align: left;
  }

  .stage-item.on {
    border-color: var(--gold);
    background: linear-gradient(90deg, var(--accent-deep), rgba(0, 0, 0, 0.35));
  }

  .id {
    font-weight: 800;
    color: var(--gold);
    width: 6cqh;
  }

  .name {
    flex: 1;
  }

  .clear {
    font-size: 0.7em;
    background: var(--sou);
    padding: 0.2cqh 1cqh;
    border-radius: 0.6cqh;
  }

  .first {
    font-size: 0.7em;
    color: #9fe0ff;
  }

  .team {
    position: absolute;
    top: 10cqh;
    left: 52cqh;
    right: 3cqh;
    bottom: 3cqh;
    display: flex;
    flex-direction: column;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .head div {
    display: flex;
    flex-direction: column;
  }

  .head small {
    font-size: 0.75em;
    opacity: 0.85;
  }

  .head .btn {
    font-size: 1.3em;
    padding: 1.4cqh 5cqh;
  }

  .label {
    margin: 1.4cqh 0 0.8cqh;
    font-size: 0.8em;
    color: var(--gold);
  }

  .slots,
  .pool {
    display: flex;
    gap: 1.6cqh;
  }

  .pool {
    flex-wrap: wrap;
    overflow-y: auto;
    padding: 1cqh 0.4cqh;
    flex: 1;
    align-content: flex-start;
  }

  .empty {
    width: 15cqh;
    height: 21cqh;
    border: 0.3cqh dashed var(--gold-deep);
    border-radius: 1cqh;
    display: grid;
    place-items: center;
    opacity: 0.6;
  }
</style>
