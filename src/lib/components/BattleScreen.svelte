<script>
  // 戰鬥畫面：引擎（wasm）在這台裝置上跑，玩家的每個指令都記下來，打完送給 Server 重算、發獎勵。
  // 操作：點一位還沒行動的隊友 → 選普攻／技能／大招 → 技能和大招點要花的牌 → 點目標 → 出手。
  import { onMount, onDestroy } from 'svelte';
  import { t } from '../i18n.svelte.js';
  import { session, finishBattle } from '../api.svelte.js';
  import { loadEngine, BattleSession } from '../engine.js';
  import { COSTUMES, SKILLS, ENEMIES, SUIT_NAME, characterOf, skillText, skillCost } from '../data.js';
  import { portrait } from '../characters.js';
  import Tile from './Tile.svelte';

  let { ticket, onexit } = $props();

  let battle = null;
  let view = $state(null);
  let log = $state([]);
  let pops = $state([]);
  let banner = $state('');
  let playing = $state(false);
  let auto = $state(false);
  let fast = $state(false);
  let result = $state(null);
  let sel = $state({ actor: null, kind: null, tiles: [], target: null });
  let destroyed = false;

  const KIND_INDEX = { basic: 0, skill: 1, ultimate: 2 };
  const LETTERS = ['A', 'B', 'C', 'D', 'E'];

  onMount(async () => {
    await loadEngine();
    battle = new BattleSession(JSON.stringify(ticket.setup), ticket.seed);
    view = JSON.parse(battle.state());
    await play(JSON.parse(battle.events()), view);
  });

  onDestroy(() => {
    destroyed = true;
    battle?.free();
  });

  // ---- 名稱 ----

  const allyName = (i) => t(characterOf(view.allies[i].id).name);
  function enemyName(i) {
    const e = view.enemies[i];
    const name = t(ENEMIES.get(e.id).name);
    const same = view.enemies.filter((x) => x.id === e.id).length > 1;
    return same ? `${name} ${LETTERS[i]}` : name;
  }
  const nameOf = (side, i) => (side === 'ally' ? allyName(i) : enemyName(i));

  // ---- 牌型（跟 legends-core 的 tiles::meld_of 一樣，畫面提示用） ----

  function isMeld(ts) {
    if (ts.length !== 3) return false;
    const n = ts.filter((x) => x.suit !== 'honor').sort((a, b) => a.rank - b.rank);
    if (n.some((x) => x.suit !== n[0].suit)) return false;
    if (n.length <= 1) return true;
    if (n.length === 2) return n[1].rank - n[0].rank <= 2;
    const [a, b, c] = n.map((x) => x.rank);
    return (a === b && b === c) || (a + 1 === b && b + 1 === c);
  }
  const enemyReady = (e) => e.skills.length > 1 && isMeld(e.hand);

  /** 手牌裡第一組可以放大招的三張 */
  function findMeld(hand) {
    for (let i = 0; i < hand.length; i++)
      for (let j = i + 1; j < hand.length; j++)
        for (let k = j + 1; k < hand.length; k++) if (isMeld([hand[i], hand[j], hand[k]])) return [i, j, k];
    return null;
  }

  // ---- 選擇指令 ----

  const actor = $derived(view && sel.actor !== null ? view.allies[sel.actor] : null);
  const skill = $derived(actor && sel.kind ? SKILLS.get(actor.skills[KIND_INDEX[sel.kind]]) : null);
  const needTiles = $derived(sel.kind === 'skill' ? skill.cost : sel.kind === 'ultimate' ? 3 : 0);
  const targetSide = $derived(skill?.target === 'enemy' ? 'enemy' : skill?.target === 'ally' ? 'ally' : null);
  const canAct = $derived(view?.phase === 'command' && !playing && !auto);

  function payable(tile) {
    if (sel.kind === 'ultimate') return true;
    return actor.suit === 'honor' || tile.suit === 'honor' || tile.suit === actor.suit;
  }

  function command() {
    const action = { kind: sel.kind, target: sel.target };
    if (sel.kind === 'skill') action.tiles = [...sel.tiles];
    if (sel.kind === 'ultimate') action.tiles = [...sel.tiles];
    return { actor: sel.actor, action };
  }

  const valid = $derived.by(() => {
    if (!battle || sel.actor === null || !sel.kind || sel.tiles.length !== needTiles) return false;
    try {
      battle.check(JSON.stringify(command()));
      return true;
    } catch {
      return false;
    }
  });

  function defaultTarget(side) {
    if (side === 'enemy') {
      const ready = view.enemies.findIndex((e) => e.hp > 0 && enemyReady(e));
      return ready >= 0 ? ready : view.enemies.findIndex((e) => e.hp > 0);
    }
    if (side === 'ally') {
      let best = -1;
      view.allies.forEach((a, i) => {
        if (a.hp > 0 && (best < 0 || a.hp / a.max_hp < view.allies[best].hp / view.allies[best].max_hp)) best = i;
      });
      return best;
    }
    return null;
  }

  function pickActor(i) {
    if (!canAct) return;
    if (targetSide === 'ally' && sel.actor !== null) {
      if (view.allies[i].hp > 0) sel.target = i;
      return;
    }
    if (!view.pending.includes(i)) return;
    sel = { actor: i, kind: null, tiles: [], target: null };
  }

  function pickKind(kind) {
    const s = SKILLS.get(actor.skills[KIND_INDEX[kind]]);
    const side = s.target === 'enemy' ? 'enemy' : s.target === 'ally' ? 'ally' : null;
    let tiles = [];
    if (kind === 'ultimate') tiles = findMeld(view.hand) ?? [];
    sel = { ...sel, kind, tiles, target: side ? defaultTarget(side) : null };
  }

  function pickTile(i) {
    if (!canAct || !needTiles) return;
    if (sel.tiles.includes(i)) sel.tiles = sel.tiles.filter((x) => x !== i);
    else if (sel.tiles.length < needTiles && payable(view.hand[i])) sel.tiles = [...sel.tiles, i];
  }

  function pickEnemy(i) {
    if (canAct && targetSide === 'enemy' && view.enemies[i].hp > 0) sel.target = i;
  }

  // ---- 執行與演出 ----

  async function send(cmd) {
    let events;
    try {
      events = JSON.parse(battle.apply(JSON.stringify(cmd)));
    } catch (e) {
      session.error = t('不能這樣出手（{why}）', { why: String(e?.message ?? e) });
      return;
    }
    sel = { actor: null, kind: null, tiles: [], target: null };
    const final = JSON.parse(battle.state());
    await play(events, final);
    view = final;
    if (final.phase !== 'command') await finish();
  }

  const wait = (ms) => new Promise((r) => setTimeout(r, fast ? ms / 2.5 : ms));
  let popKey = 0;
  function pop(side, slot, text, kind) {
    const key = ++popKey;
    pops = [...pops, { key, side, slot, text, kind }];
    setTimeout(() => (pops = pops.filter((p) => p.key !== key)), 1000);
  }
  const say = (text) => (log = [...log.slice(-7), text]);
  const tileText = (ts) => ts.map((x) => (x.suit === 'honor' ? '東南西北中發白'[x.rank - 1] : `${'一二三四五六七八九'[x.rank - 1]}${SUIT_NAME[x.suit]}`)).join(' ');

  /** 一個一個播事件，邊播邊更新血條；final 是播完後的狀態（換波次時拿新敵人） */
  async function play(events, final) {
    playing = true;
    for (const e of events) {
      if (destroyed) return;
      switch (e.type) {
        case 'round':
          say(t('── 第 {n} 回合 ──', { n: e.n }));
          if (view) view.round = e.n;
          break;
        case 'wave':
          if (e.index > 0) {
            view.enemies = final.enemies.map((x) => ({ ...x, hp: x.max_hp, shield: 0 }));
            view.wave = e.index;
            banner = t('第 {n} 波', { n: e.index + 1 });
            await wait(700);
            banner = '';
          }
          break;
        case 'draw':
          if (e.tiles.length) say(t('摸牌：{tiles}', { tiles: tileText(e.tiles) }));
          break;
        case 'ready':
          say(t('{name} 聽牌了！', { name: enemyName(e.slot) }));
          break;
        case 'act': {
          const s = SKILLS.get(e.skill);
          say(t('{name}：{skill}', { name: nameOf(e.side, e.slot), skill: t(s.name) }));
          if (s.kind === 'ultimate') {
            banner = t(s.name);
            await wait(500);
            banner = '';
          }
          await wait(260);
          break;
        }
        case 'damage':
          (e.side === 'ally' ? view.allies : view.enemies)[e.slot].hp = e.hp;
          pop(e.side, e.slot, `-${e.amount}`, 'dmg');
          await wait(120);
          break;
        case 'heal':
          (e.side === 'ally' ? view.allies : view.enemies)[e.slot].hp = e.hp;
          pop(e.side, e.slot, `+${e.amount}`, 'heal');
          await wait(80);
          break;
        case 'shield':
          (e.side === 'ally' ? view.allies : view.enemies)[e.slot].shield += e.amount;
          pop(e.side, e.slot, t('護盾'), 'buff');
          break;
        case 'buff':
          pop(e.side, e.slot, `${t({ atk: '攻擊', def: '防禦', spd: '速度' }[e.stat])}${e.pct > 0 ? '↑' : '↓'}`, e.pct > 0 ? 'buff' : 'debuff');
          break;
        case 'steal':
          if (e.tiles.length) say(t('{tiles} 被拿掉了', { tiles: tileText(e.tiles) }));
          if (e.side === 'enemy') view.enemies[e.slot].hand = view.enemies[e.slot].hand.slice(0, -e.tiles.length);
          break;
        case 'fizzle':
          say(t('{name} 要用的牌被拿走了，改成普攻', { name: allyName(e.slot) }));
          break;
        case 'down':
          say(t('{name} 倒下了', { name: nameOf(e.side, e.slot) }));
          break;
        case 'combo':
          banner = t('合技！');
          say(t('這回合打出一順一刻：全隊合技！'));
          await wait(700);
          banner = '';
          break;
        case 'won':
          banner = t('勝利');
          break;
        case 'lost':
          banner = t('敗北');
          break;
      }
    }
    playing = false;
  }

  async function runAuto() {
    while (auto && view?.phase === 'command' && !destroyed) {
      while (playing) await wait(50);
      const c = battle.auto();
      if (!c) break;
      await send(JSON.parse(c));
    }
  }

  function toggleAuto() {
    auto = !auto;
    sel = { actor: null, kind: null, tiles: [], target: null };
    if (auto) runAuto();
  }

  async function finish() {
    auto = false;
    result = { won: view.phase === 'won', pending: true };
    try {
      const r = await finishBattle(ticket.ticket, JSON.parse(battle.log()), battle.digest());
      result = { won: r.outcome.won, rewards: r.rewards, rounds: r.outcome.rounds, desync: r.desync };
    } catch {
      result = { won: view.phase === 'won', failed: true };
    }
  }

  function retreat() {
    if (confirm(t('撤退的話這場不會有獎勵，確定嗎？'))) onexit();
  }

  const hpPct = (f) => `${Math.max(0, (f.hp / f.max_hp) * 100)}%`;
  const shieldPct = (f) => `${Math.min(100, (f.shield / f.max_hp) * 100)}%`;
</script>

{#if view}
  <div class="battle">
    <header>
      <span class="round">{t('第 {r} 回合', { r: view.round })}・{t('第 {w}／{n} 波', { w: view.wave + 1, n: view.waves })}</span>
      <div class="tools">
        <button type="button" class:on={auto} onclick={toggleAuto} disabled={view.phase !== 'command'}>{t('自動')}</button>
        <button type="button" class:on={fast} onclick={() => (fast = !fast)}>{t('2 倍速')}</button>
        <button type="button" onclick={retreat} disabled={view.phase !== 'command'}>{t('撤退')}</button>
      </div>
    </header>

    <section class="enemies">
      {#each view.enemies as e, i (i + '-' + view.wave)}
        <button
          type="button"
          class="enemy suit-{e.suit}"
          class:down={e.hp <= 0}
          class:target={targetSide === 'enemy' && sel.target === i}
          class:pickable={canAct && targetSide === 'enemy' && e.hp > 0}
          onclick={() => pickEnemy(i)}
        >
          {#if enemyReady(e) && e.hp > 0}<span class="ready">{t('聽牌')}</span>{/if}
          <span class="ename">{enemyName(i)}</span>
          <span class="token">{t(SUIT_NAME[e.suit])}</span>
          <span class="bar"><i style:width={hpPct(e)}></i><s style:width={shieldPct(e)}></s></span>
          <span class="hpnum">{e.hp}／{e.max_hp}</span>
          <span class="ehand">
            {#each e.hand as tile, k (k)}<Tile {tile} size="sm" disabled />{/each}
          </span>
          {#each pops.filter((p) => p.side === 'enemy' && p.slot === i) as p (p.key)}<b class="pop {p.kind}">{p.text}</b>{/each}
        </button>
      {/each}
    </section>

    <ol class="log" aria-live="polite">
      {#each log as line, i (i + line)}<li>{line}</li>{/each}
    </ol>

    <section class="allies">
      {#each view.allies as a, i (i)}
        {@const def = COSTUMES.get(a.id)}
        <button
          type="button"
          class="ally rarity-{def.rarity} suit-{a.suit}"
          class:down={a.hp <= 0}
          class:acted={a.acted && a.hp > 0}
          class:active={sel.actor === i}
          class:target={targetSide === 'ally' && sel.target === i}
          onclick={() => pickActor(i)}
        >
          <img src={portrait(def.character, a.hp <= 0 ? 'sad' : view.phase === 'won' ? 'win' : 'normal')} alt="" draggable="false" />
          <span class="token">{t(SUIT_NAME[a.suit])}</span>
          <span class="aname">{allyName(i)}<small>{t(def.name)}</small></span>
          <span class="bar"><i style:width={hpPct(a)}></i><s style:width={shieldPct(a)}></s></span>
          <span class="hpnum">{a.hp}</span>
          {#if a.acted && a.hp > 0}<span class="done">{t('已行動')}</span>{/if}
          {#each pops.filter((p) => p.side === 'ally' && p.slot === i) as p (p.key)}<b class="pop {p.kind}">{p.text}</b>{/each}
        </button>
      {/each}
    </section>

    <section class="hand">
      <p class="label">{t('牌氣（{n}／8）', { n: view.hand.length })}</p>
      <div class="tiles">
        {#each view.hand as tile, i (i + '-' + tile.suit + tile.rank)}
          {#if canAct && needTiles}
            <Tile {tile} selected={sel.tiles.includes(i)} disabled={!sel.tiles.includes(i) && !payable(tile)} onclick={() => pickTile(i)} />
          {:else}
            <Tile {tile} />
          {/if}
        {/each}
      </div>
    </section>

    <section class="command panel">
      {#if auto}
        <p>{t('自動戰鬥中…')}</p>
      {:else if !actor}
        <p>{t('點一位還沒行動的隊友。敵人比他快的話，會在他之前出手。')}</p>
      {:else}
        <div class="kinds">
          {#each ['basic', 'skill', 'ultimate'] as k (k)}
            {@const s = SKILLS.get(actor.skills[KIND_INDEX[k]])}
            <button type="button" class:on={sel.kind === k} disabled={!canAct} onclick={() => pickKind(k)}>
              <b>{[t('普攻'), t('技能'), t('大招')][KIND_INDEX[k]]}</b>{t(s.name)}
            </button>
          {/each}
        </div>
        {#if skill}
          <p class="desc">{skillCost(skill)}・{skillText(skill)}</p>
          {#if needTiles}<p class="desc">{t('已選 {a}／{b} 張', { a: sel.tiles.length, b: needTiles })}</p>{/if}
        {/if}
        <button class="btn go" type="button" disabled={!valid || !canAct} onclick={() => send(command())}>{t('出手')}</button>
      {/if}
    </section>

    {#if banner}<div class="banner">{banner}</div>{/if}

    {#if result}
      <div class="scrim" role="dialog" aria-modal="true">
        <div class="panel res">
          <h3>{result.won ? t('勝利！') : t('敗北…')}</h3>
          {#if result.pending}
            <p>{t('伺服器驗證中…')}</p>
          {:else if result.failed}
            <p>{session.error || t('結算失敗')}</p>
          {:else if result.won}
            <p>{t('{n} 回合過關', { n: result.rounds })}</p>
            <p>{t('金幣 +{g}・練習筆記 +{n}', { g: result.rewards.gold, n: result.rewards.notes })}</p>
            {#if result.rewards.first}<p class="first">{t('首次通關：星屑 +{s}', { s: result.rewards.stardust })}</p>{/if}
          {:else}
            <p>{t('練等級或換隊伍再來一次吧。')}</p>
          {/if}
          {#if result.desync}<p class="warn">{t('這台裝置跟伺服器的結果不一樣，獎勵以伺服器為準。')}</p>{/if}
          <button class="btn" type="button" disabled={result.pending} onclick={onexit}>{t('返回')}</button>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .battle {
    position: absolute;
    inset: 0;
  }

  header {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 7cqh;
    padding: 0 2.4cqh;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: rgba(0, 0, 0, 0.45);
  }

  .round {
    color: var(--gold);
    font-weight: 700;
  }

  .tools {
    display: flex;
    gap: 1cqh;
  }

  .tools button {
    background: rgba(0, 0, 0, 0.35);
    border: 0.2cqh solid var(--gold-deep);
    border-radius: 0.8cqh;
    padding: 0.5cqh 1.8cqh;
  }

  .tools button.on {
    background: var(--accent);
    border-color: var(--gold);
  }

  .enemies {
    position: absolute;
    top: 9cqh;
    left: 4cqh;
    right: 46cqh;
    height: 31cqh;
    display: flex;
    justify-content: center;
    gap: 2cqh;
  }

  .enemy {
    position: relative;
    width: 24cqh;
    border: var(--line) solid rgba(0, 0, 0, 0.5);
    border-radius: 1.2cqh;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.55), rgba(0, 0, 0, 0.25));
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1.2cqh;
    gap: 0.6cqh;
    cursor: default;
  }

  .enemy.pickable {
    cursor: pointer;
    border-color: rgba(227, 181, 82, 0.5);
  }

  .enemy.target {
    border-color: var(--gold);
    box-shadow: 0 0 2cqh rgba(227, 181, 82, 0.7);
  }

  .enemy.down,
  .ally.down {
    opacity: 0.35;
    filter: grayscale(1);
  }

  .ename {
    font-weight: 700;
    font-size: 0.85em;
  }

  .enemy .token {
    width: 9cqh;
    height: 9cqh;
    font-size: 2em;
    border-radius: 50%;
  }

  .token {
    display: grid;
    place-items: center;
    background: var(--ivory);
    color: var(--suit);
    font-weight: 900;
    border: 0.3cqh solid var(--suit);
  }

  .ready {
    position: absolute;
    top: -1.4cqh;
    right: -1cqh;
    background: var(--accent);
    border: 0.25cqh solid var(--gold);
    border-radius: 0.8cqh;
    padding: 0.2cqh 1cqh;
    font-weight: 800;
    font-size: 0.75em;
    animation: blink 0.8s infinite alternate;
  }

  @keyframes blink {
    to {
      filter: brightness(1.5);
    }
  }

  .bar {
    position: relative;
    width: 100%;
    height: 1.4cqh;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 1cqh;
    overflow: hidden;
  }

  .bar i,
  .bar s {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    transition: width 0.25s;
  }

  .bar i {
    background: var(--hp);
  }

  .bar s {
    background: var(--shield);
    opacity: 0.75;
  }

  .hpnum {
    font-size: 0.65em;
    font-variant-numeric: tabular-nums;
    opacity: 0.85;
  }

  .ehand {
    display: flex;
    gap: 0.4cqh;
    min-height: 5.4cqh;
  }

  .log {
    position: absolute;
    top: 9cqh;
    right: 2.4cqh;
    width: 42cqh;
    height: 31cqh;
    margin: 0;
    padding: 1cqh 1.6cqh;
    list-style: none;
    background: rgba(0, 0, 0, 0.35);
    border-radius: 1cqh;
    font-size: 0.7em;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 0.3cqh;
  }

  .allies {
    position: absolute;
    top: 42cqh;
    left: 4cqh;
    right: 4cqh;
    height: 27cqh;
    display: flex;
    justify-content: center;
    gap: 2cqh;
  }

  .ally {
    position: relative;
    width: 32cqh;
    padding: 0;
    border: 0.4cqh solid var(--rarity);
    border-radius: 1.2cqh;
    overflow: hidden;
    background: radial-gradient(circle at 30% 30%, #2d6d55 0%, #0a2a1f 100%);
    text-align: left;
  }

  .ally img {
    position: absolute;
    right: -2cqh;
    top: 0;
    height: 30cqh;
    object-fit: cover;
    object-position: 50% 0;
  }

  .ally.active {
    border-color: var(--gold);
    transform: translateY(-1.2cqh);
    box-shadow: 0 0 2.4cqh rgba(227, 181, 82, 0.8);
  }

  .ally.target {
    outline: 0.5cqh solid var(--hp);
  }

  .ally.acted img {
    filter: brightness(0.55);
  }

  .ally .token {
    position: absolute;
    top: 1cqh;
    left: 1cqh;
    width: 4cqh;
    height: 4cqh;
    border-radius: 0.6cqh;
  }

  .aname {
    position: absolute;
    left: 1.2cqh;
    bottom: 6cqh;
    display: flex;
    flex-direction: column;
    font-weight: 800;
    text-shadow: 0 0.3cqh 0.4cqh #000;
  }

  .aname small {
    font-size: 0.6em;
    color: var(--gold);
  }

  .ally .bar {
    position: absolute;
    left: 1.2cqh;
    right: 1.2cqh;
    bottom: 2.6cqh;
    width: auto;
  }

  .ally .hpnum {
    position: absolute;
    left: 1.2cqh;
    bottom: 0.4cqh;
  }

  .done {
    position: absolute;
    top: 1cqh;
    right: 1cqh;
    background: rgba(0, 0, 0, 0.7);
    border-radius: 0.6cqh;
    padding: 0.2cqh 1cqh;
    font-size: 0.65em;
  }

  .pop {
    position: absolute;
    left: 50%;
    top: 30%;
    transform: translateX(-50%);
    font-size: 1.4em;
    font-weight: 900;
    pointer-events: none;
    text-shadow: 0 0.3cqh 0 #000;
    animation: rise 1s ease-out forwards;
  }

  .pop.dmg {
    color: #ffd2c7;
  }
  .pop.heal {
    color: var(--hp);
  }
  .pop.buff {
    color: var(--shield);
  }
  .pop.debuff {
    color: #ff9f6b;
  }

  @keyframes rise {
    from {
      opacity: 1;
      margin-top: 0;
    }
    to {
      opacity: 0;
      margin-top: -6cqh;
    }
  }

  .hand {
    position: absolute;
    left: 4cqh;
    bottom: 3cqh;
    width: 84cqh;
  }

  .label {
    margin: 0 0 1cqh;
    font-size: 0.75em;
    color: var(--gold);
  }

  .tiles {
    display: flex;
    gap: 1cqh;
    min-height: 10cqh;
    align-items: flex-end;
  }

  .command {
    position: absolute;
    right: 3cqh;
    bottom: 3cqh;
    width: 82cqh;
    height: 24cqh;
    padding: 1.6cqh 2cqh;
    font-size: 0.85em;
    display: flex;
    flex-direction: column;
  }

  .command p {
    margin: 0.4cqh 0;
  }

  .kinds {
    display: flex;
    gap: 1cqh;
  }

  .kinds button {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--ivory-deep);
    color: var(--ink);
    border: 0.25cqh solid var(--gold-deep);
    border-radius: 0.8cqh;
    padding: 0.6cqh;
  }

  .kinds button.on {
    background: var(--accent);
    color: var(--ivory);
    border-color: var(--gold);
  }

  .desc {
    font-size: 0.9em;
    color: #4a4135;
  }

  .go {
    margin-top: auto;
    align-self: flex-end;
    padding: 1cqh 5cqh;
  }

  .banner {
    position: absolute;
    top: 36cqh;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 3.4em;
    font-weight: 900;
    letter-spacing: 0.3em;
    color: var(--ivory);
    -webkit-text-stroke: 0.4cqh var(--accent-deep);
    paint-order: stroke fill;
    text-shadow: 0 0 3cqh rgba(227, 181, 82, 0.9);
    pointer-events: none;
    animation: pop-in 0.25s ease-out;
    z-index: 10;
  }

  @keyframes pop-in {
    from {
      transform: scale(1.6);
      opacity: 0;
    }
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    display: grid;
    place-items: center;
    z-index: 20;
  }

  .res {
    width: 60cqh;
    padding: 3cqh;
    text-align: center;
  }

  .res h3 {
    margin: 0 0 1cqh;
    font-size: 2em;
  }

  .first {
    color: var(--pin);
    font-weight: 800;
  }

  .warn {
    color: var(--accent-deep);
    font-size: 0.8em;
  }
</style>
