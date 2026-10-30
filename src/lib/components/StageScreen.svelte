<script>
  // 出擊：先選關卡（關卡列表），點進去才是編隊（最多四人，同一位角色只能上一個版本）。
  // 開打時 Server 用它記錄的等級組隊、發戰鬥票。
  import { Panel, Button, SectionBar, Tag } from 'bluebolt-ui';
  import { session, startBattle } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, STAGES, ENEMIES, ELEMENT_NAME } from '../data.js';
  import TopBar from './TopBar.svelte';
  import UnitCard from './UnitCard.svelte';
  import { sceneArt, stageScene } from '../art.js';

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
  /** list：關卡列表；team：這一關的編隊 */
  let view = $state('list');
  const pick = (id) => ((stage = id), (view = 'team'));
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

<img class="backdrop" src={view === 'list' ? sceneArt('bg_entrance') : stageScene(stage)} alt="" draggable="false" />
<span class="veil" aria-hidden="true"></span>

<TopBar title={view === 'list' ? t('出擊') : t('編隊')} en={view === 'list' ? 'Sortie' : 'Party'} num="01" onback={view === 'list' ? onback : () => (view = 'list')} />

{#if view === 'list'}
  <section class="chapter">
    <SectionBar title={t('第一章　新傳說的入口')} en="Chapter 1" />
    <ol class="map">
      {#each STAGES as s, i (s.id)}
        {@const boss = s.waves.flat().some((id) => ENEMIES.get(id).boss)}
        <li style:--i={i} class:boss>
          <button type="button" class:cleared={clears[s.id] !== undefined} class:next={unlocked(i) && clears[s.id] === undefined} disabled={!unlocked(i)} onclick={() => pick(s.id)}>
            <span class="code">{s.id}</span>
            <span class="name">{t(s.name)}</span>
            <span class="meta">
              {#if s.env}<em>{t(s.env.name)}</em>{/if}
              {#if boss}<em class="b">BOSS</em>{/if}
            </span>
            <span class="state">{#if clears[s.id] !== undefined}CLEAR{:else if unlocked(i)}✦{s.rewards.first_stardust}{:else}LOCK{/if}</span>
          </button>
        </li>
      {/each}
    </ol>
  </section>
{:else}
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
{/if}

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  /* 上面亮、下面淡，讓字和卡片讀得清楚 */
  .veil {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgb(243 246 253 / 0.88) 0 14%, rgb(243 246 253 / 0.35) 45%, rgb(11 15 26 / 0.35));
  }

  .chapter {
    position: absolute;
    left: 40px;
    right: 40px;
    top: 112px;
    bottom: 28px;
  }

  /* 關卡像地圖一樣兩排、往右上爬 */
  .map {
    list-style: none;
    margin: 18px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    grid-auto-flow: row;
    gap: 22px 18px;
  }

  .map li {
    transform: translateY(calc((4 - var(--i) % 5) * 10px));
    animation: in var(--bb-dur-4) var(--bb-ease) both;
    animation-delay: calc(var(--i) * 40ms);
  }

  @keyframes in {
    from {
      transform: translateY(calc(40px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .map button {
    position: relative;
    width: 100%;
    height: 210px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px 18px;
    border: 0;
    text-align: left;
    color: #fff;
    background: var(--bb-facets), linear-gradient(180deg, #2a3350, #0b0f1a);
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 18px), calc(100% - 18px) 100%, 0 100%);
    box-shadow: inset 0 -4px 0 var(--bb-blue);
    transition: transform var(--bb-dur-2) var(--bb-ease);
  }

  .map button:not(:disabled):hover {
    transform: translateY(-6px);
  }

  .map button.next {
    background: var(--bb-face-blue);
    box-shadow: inset 0 -5px 0 var(--bb-yellow);
  }

  .map button.next::after {
    content: 'NEXT';
    position: absolute;
    top: 10px;
    right: 12px;
    padding: 1px 8px;
    background: var(--bb-yellow);
    color: var(--bb-on-yellow);
    font: italic 700 12px var(--bb-font-cond);
    letter-spacing: 0.15em;
  }

  .map li.boss button {
    background: var(--bb-facets), linear-gradient(180deg, #4a1530, #1b0a14);
    box-shadow: inset 0 -5px 0 #ff4d5e;
  }

  .map button:disabled {
    opacity: 0.4;
  }

  .code {
    font: italic 400 52px/1 var(--bb-font-display);
    color: transparent;
    -webkit-text-stroke: 1.5px #fff;
  }

  .name {
    font-size: 20px;
    font-weight: 900;
    flex: 1;
  }

  .meta {
    display: flex;
    gap: 6px;
  }

  .meta em {
    font-style: normal;
    font-size: 11px;
    font-weight: 800;
    padding: 1px 8px;
    background: rgb(255 255 255 / 0.18);
  }

  .meta em.b {
    background: #ff4d5e;
  }

  .state {
    font: italic 700 14px var(--bb-font-cond);
    letter-spacing: 0.15em;
    color: var(--bb-yellow);
  }

  .cleared .state {
    color: var(--gale);
  }

  .side {
    position: absolute;
    left: 40px;
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
  p.meta {
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
