<script>
  // 戰鬥畫面：引擎（wasm）在這台裝置上跑，玩家的每個指令都記下來，打完送給 Server 重算、發獎勵。
  // 操作：輪到某位隊友時選一招（S1～S3）→ 點目標 → 出手。聲援 3 格以上可以「接棒」，指定一位隊友緊接著出手。
  // 外觀是暫時的；bluebolt-ui 的版本另外做。
  import { onMount, onDestroy } from 'svelte';
  import { t } from '../i18n.svelte.js';
  import { session, finishBattle } from '../api.svelte.js';
  import { loadEngine, BattleSession } from '../engine.js';
  import { COSTUMES, SKILLS, ENEMIES, STAGES, ELEMENT_NAME, STATUS_NAME, isBuff, characterOf, skillAt, skillText, cooldownText } from '../data.js';
  import { portrait } from '../characters.js';

  let { ticket, onexit } = $props();

  let battle = null;
  let view = $state(null);
  let log = $state([]);
  let pops = $state([]);
  let banner = $state(null);
  let playing = $state(false);
  let auto = $state(false);
  let fast = $state(false);
  let result = $state(null);
  let sel = $state({ skill: null, target: null, relay: null, relayMode: false });
  let acting = $state(null);
  let destroyed = false;

  const LETTERS = ['A', 'B', 'C', 'D', 'E'];
  const SHORT = {
    atk_up: '攻↑', def_up: '防↑', spd_up: '速↑', crit_up: '暴↑', immune: '免疫', counter: '反擊',
    atk_down: '攻↓', def_down: '防↓', spd_down: '速↓', hit_down: '命↓', burn: '灼燒', stun: '暈眩', taunt: '挑釁', silence: '封印', no_heal: '禁療',
  };
  const stage = STAGES.find((s) => s.id === ticket.setup.stage);

  onMount(async () => {
    await loadEngine();
    battle = new BattleSession(JSON.stringify(ticket.setup), ticket.seed);
    const events = JSON.parse(battle.events());
    const final = JSON.parse(battle.state());
    view = final;
    await play(events, final);
    view = final;
    suggest();
    if (final.phase !== 'command') await finish();
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
  const unitOf = (side, i) => (side === 'ally' ? view.allies : view.enemies)[i];

  // ---- 選擇指令 ----

  const current = $derived(view?.phase === 'command' ? view.current : null);
  const actor = $derived(current !== null && current !== undefined ? view.allies[current] : null);
  /** 這位隊友第 i 招（套用技能等級） */
  const skillOf = (allyIndex, i) => skillAt(SKILLS.get(view.allies[allyIndex].skill_ids[i]), ticket.setup.team[allyIndex].skills?.[i] ?? 1);
  const skill = $derived(actor && sel.skill !== null ? skillOf(current, sel.skill) : null);
  const targetSide = $derived(skill?.target === 'enemy' ? 'enemy' : skill?.target === 'ally' ? 'ally' : null);
  const canAct = $derived(current !== null && current !== undefined && !playing && !auto && !result);

  function command() {
    const c = { actor: current, skill: sel.skill };
    if (targetSide) c.target = sel.target;
    if (sel.relay !== null) c.relay = sel.relay;
    return c;
  }

  function check(c) {
    try {
      battle.check(JSON.stringify(c));
      return '';
    } catch (e) {
      return String(e?.message ?? e);
    }
  }

  const problem = $derived.by(() => {
    if (!battle || !canAct || sel.skill === null) return 'none';
    return check(command());
  });

  /** 輪到我方時，先照自動戰鬥的建議選好（不含接棒） */
  function suggest() {
    if (current === null || current === undefined || !battle) return;
    const c = JSON.parse(battle.auto() ?? 'null');
    sel = { skill: c?.skill ?? 0, target: c?.target ?? null, relay: null, relayMode: false };
  }

  function usable(i) {
    return actor && check({ actor: current, skill: i, target: firstTarget(skillOf(current, i)) }) === '';
  }

  function firstTarget(s) {
    if (s.target === 'enemy') {
      const forced = actor.statuses.find((x) => x.kind === 'taunt' && x.source?.[0] === 'enemy');
      if (forced && view.enemies[forced.source[1]].hp > 0) return forced.source[1];
      const keep = sel.target !== null && targetSide === 'enemy' && view.enemies[sel.target]?.hp > 0;
      return keep ? sel.target : view.enemies.findIndex((e) => e.hp > 0);
    }
    if (s.target === 'ally') {
      let best = -1;
      view.allies.forEach((a, i) => {
        if (a.hp > 0 && (best < 0 || a.hp / a.max_hp < view.allies[best].hp / view.allies[best].max_hp)) best = i;
      });
      return best;
    }
    return undefined;
  }

  function pickSkill(i) {
    if (!canAct) return;
    const s = skillOf(current, i);
    sel = { ...sel, skill: i, target: firstTarget(s) ?? null, relayMode: false };
  }

  function pickEnemy(i) {
    if (!canAct || view.enemies[i].hp <= 0) return;
    if (targetSide !== 'enemy') pickSkill(0);
    sel.target = i;
  }

  function pickAlly(i) {
    if (!canAct || view.allies[i].hp <= 0) return;
    if (sel.relayMode) {
      sel = { ...sel, relay: i === current || sel.relay === i ? null : i, relayMode: false };
    } else if (targetSide === 'ally') {
      sel.target = i;
    }
  }

  function toggleRelay() {
    if (sel.relay !== null) sel = { ...sel, relay: null, relayMode: false };
    else sel = { ...sel, relayMode: !sel.relayMode };
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
    sel = { skill: null, target: null, relay: null, relayMode: false };
    const final = JSON.parse(battle.state());
    await play(events, final);
    view = final;
    if (final.phase !== 'command') await finish();
    else if (!auto) suggest();
  }

  const wait = (ms) => new Promise((r) => setTimeout(r, fast ? ms / 2.5 : ms));
  let popKey = 0;
  function pop(side, slot, text, kind) {
    const key = ++popKey;
    pops = [...pops, { key, side, slot, text, kind }];
    setTimeout(() => (pops = pops.filter((p) => p.key !== key)), 1100);
  }
  const say = (text) => (log = [...log.slice(-5), text]);

  /** 一個一個播事件，邊播邊更新畫面；final 是播完後的狀態（換波次時拿新敵人） */
  async function play(events, final) {
    playing = true;
    for (const e of events) {
      if (destroyed) return;
      const u = e.side ? unitOf(e.side, e.unit) : null;
      switch (e.type) {
        case 'wave':
          if (e.wave > 0) {
            view.enemies = final.enemies.map((x) => ({ ...x, hp: x.max_hp, shield: 0, statuses: [] }));
            view.wave = e.wave;
            banner = { text: t('第 {n} 波', { n: e.wave + 1 }) };
            await wait(700);
            banner = null;
          }
          break;
        case 'turn':
          acting = { side: e.side, unit: e.unit };
          break;
        case 'skill': {
          const s = SKILLS.get(e.skill);
          const via = { counter: t('反擊'), link: t('連攜'), pursuit: t('追擊') }[e.via] ?? '';
          say(`${nameOf(e.side, e.unit)}：${t(s.name)}${via ? `（${via}）` : ''}`);
          if (e.slot === 2 && !e.via && e.side === 'ally') {
            banner = { text: t(s.name), portrait: COSTUMES.get(u.id).character };
            await wait(650);
            banner = null;
          } else if (e.slot === 2 && !e.via) {
            banner = { text: t(s.name), enemy: true };
            await wait(500);
            banner = null;
          }
          await wait(220);
          break;
        }
        case 'trigger':
          if (e.via === 'pursuit' || e.via === 'passive') pop(e.side, e.unit, e.name ? t(e.name) : t('追擊'), 'buff');
          else pop(e.side, e.unit, e.via === 'counter' ? t('反擊！') : t('連攜！'), 'buff');
          await wait(180);
          break;
        case 'damage':
          u.hp = e.hp;
          u.shield = e.shield;
          pop(e.side, e.unit, `${e.crit ? t('暴擊 ') : ''}-${e.amount}`, e.crit ? 'crit' : e.graze ? 'graze' : 'dmg');
          await wait(110);
          break;
        case 'heal':
          u.hp = e.hp;
          if (e.amount) pop(e.side, e.unit, `+${e.amount}`, 'heal');
          await wait(70);
          break;
        case 'shield':
          u.shield = Math.max(u.shield, e.amount);
          pop(e.side, e.unit, t('護盾'), 'buff');
          break;
        case 'status': {
          const found = u.statuses.find((s) => s.kind === e.status);
          if (found) found.turns = Math.max(found.turns, e.turns);
          else u.statuses = [...u.statuses, { kind: e.status, turns: e.turns }];
          pop(e.side, e.unit, t(STATUS_NAME[e.status]), isBuff(e.status) ? 'buff' : 'debuff');
          await wait(60);
          break;
        }
        case 'resisted':
          pop(e.side, e.unit, t('抵抗'), 'graze');
          break;
        case 'expired':
        case 'removed':
          u.statuses = u.statuses.filter((s) => s.kind !== e.status);
          break;
        case 'gauge':
          u.gauge = e.gauge;
          pop(e.side, e.unit, e.pct > 0 ? t('加速') : t('延遲'), e.pct > 0 ? 'buff' : 'debuff');
          break;
        case 'charge':
          view.enemies[e.unit].charge = { turns: e.turns - 1 };
          say(t('{name} 開始蓄力「{skill}」！', { name: enemyName(e.unit), skill: t(SKILLS.get(e.skill).name) }));
          break;
        case 'charging':
          view.enemies[e.unit].charge = { turns: e.turns };
          break;
        case 'interrupted':
          view.enemies[e.unit].charge = null;
          pop('enemy', e.unit, t('打斷！'), 'buff');
          say(t('{name} 的蓄力被打斷了', { name: enemyName(e.unit) }));
          break;
        case 'break':
          view.enemies[e.unit].toughness = 0;
          pop('enemy', e.unit, t('破盾！'), 'crit');
          await wait(200);
          break;
        case 'phase':
          banner = { text: t('{name}：{phase}', { name: enemyName(e.unit), phase: t(e.name) }), enemy: true };
          await wait(800);
          banner = null;
          break;
        case 'stunned':
          pop(e.side, e.unit, t('暈眩中'), 'graze');
          await wait(200);
          break;
        case 'down':
          u.hp = 0;
          u.statuses = [];
          say(t('{name} 倒下了', { name: nameOf(e.side, e.unit) }));
          break;
        case 'cheer':
          view.cheer = e.value;
          break;
        case 'relay':
          say(t('{a} 接棒給 {b}！', { a: allyName(e.from), b: allyName(e.to) }));
          break;
        case 'won':
          banner = { text: t('勝利') };
          break;
        case 'lost':
          banner = { text: t('敗北') };
          break;
      }
    }
    acting = null;
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
    if (auto) runAuto();
    else suggest();
  }

  async function finish() {
    auto = false;
    await wait(600);
    banner = null;
    result = { won: view.phase === 'won', pending: true };
    try {
      const r = await finishBattle(ticket.ticket, JSON.parse(battle.log()), battle.digest());
      result = { won: r.outcome.won, rewards: r.rewards, outcome: r.outcome, desync: r.desync };
    } catch {
      result = { won: view.phase === 'won', failed: true };
    }
  }

  function retreat() {
    if (confirm(t('撤退的話這場不會有獎勵，確定嗎？'))) onexit();
  }

  const pct = (a, b) => `${Math.max(0, Math.min(100, (a / b) * 100))}%`;
  const isActing = (side, i) => (acting ? acting.side === side && acting.unit === i : side === 'ally' && current === i);
  const stars = $derived(
    result?.outcome
      ? [result.outcome.won, result.outcome.won && result.outcome.rounds <= stage.star_turns, result.outcome.won && result.outcome.survivors === view.allies.length]
      : [],
  );
</script>

{#if view}
  <div class="battle">
    <header>
      <span class="info">
        {t('第 {w}／{n} 波', { w: view.wave + 1, n: view.waves })}・{t('我方 {n} 回合', { n: view.turns })}
        {#if view.env}<span class="env" title={t(view.env.text)}>{t(view.env.name)}：{t(view.env.text)}</span>{/if}
      </span>
      <div class="tools">
        <button type="button" class:on={auto} onclick={toggleAuto} disabled={view.phase !== 'command'}>{t('自動')}</button>
        <button type="button" class:on={fast} onclick={() => (fast = !fast)}>{t('2 倍速')}</button>
        <button type="button" onclick={retreat} disabled={view.phase !== 'command'}>{t('撤退')}</button>
      </div>
    </header>

    <ol class="timeline" aria-label={t('出手順序')}>
      {#each view.timeline as w, k (k + w.side + w.unit)}
        {@const u = unitOf(w.side, w.unit)}
        <li class="tl tl-{w.side} el-{u.element}" class:now={k === 0}>
          {#if w.side === 'ally'}
            <img src={portrait(COSTUMES.get(u.id).character)} alt={allyName(w.unit)} draggable="false" />
          {:else}
            <span class="tok">{t(ENEMIES.get(u.id).name).slice(0, 1)}</span>
            {#if u.charge}<b class="ch">⚡</b>{/if}
          {/if}
        </li>
      {/each}
    </ol>

    <section class="field">
      <div class="allies">
        {#each view.allies as a, i (i)}
          {@const def = COSTUMES.get(a.id)}
          <button
            type="button"
            class="ally rarity-{def.rarity} el-{a.element} pos-{i}"
            class:down={a.hp <= 0}
            class:acting={isActing('ally', i)}
            class:target={targetSide === 'ally' && sel.target === i}
            class:relay={sel.relay === i}
            class:pickable={canAct && ((targetSide === 'ally' && a.hp > 0) || (sel.relayMode && i !== current && a.hp > 0))}
            onclick={() => pickAlly(i)}
          >
            <img src={portrait(def.character, a.hp <= 0 ? 'sad' : view.phase === 'won' ? 'win' : 'normal')} alt="" draggable="false" />
            <span class="el">{t(ELEMENT_NAME[a.element])}</span>
            <span class="nm">{allyName(i)}<small>{t(def.name)}</small></span>
            <span class="bars">
              <span class="bar hp"><i style:width={pct(a.hp, a.max_hp)}></i><s style:width={pct(a.shield, a.max_hp)}></s></span>
              <span class="bar gauge"><i style:width={pct(a.gauge, 1000)}></i></span>
            </span>
            <span class="hpnum">{a.hp}</span>
            <span class="sts">{#each a.statuses as s (s.kind)}<em class:bad={!isBuff(s.kind)}>{t(SHORT[s.kind])}{s.turns}</em>{/each}</span>
            {#if sel.relay === i}<span class="tag">{t('接棒')}</span>{/if}
            {#each pops.filter((p) => p.side === 'ally' && p.slot === i) as p (p.key)}<b class="pop {p.kind}">{p.text}</b>{/each}
          </button>
        {/each}
      </div>

      <div class="enemies">
        {#each view.enemies as e, i (i + '-' + view.wave)}
          <button
            type="button"
            class="enemy el-{e.element} pos-{i}"
            class:boss={e.boss}
            class:down={e.hp <= 0}
            class:acting={isActing('enemy', i)}
            class:target={targetSide === 'enemy' && sel.target === i}
            class:pickable={canAct && e.hp > 0}
            onclick={() => pickEnemy(i)}
          >
            {#if e.charge && e.hp > 0}<span class="charge">{t('蓄力 {n}', { n: e.charge.turns + 1 })}</span>{/if}
            <span class="tok">{t(ENEMIES.get(e.id).name).slice(0, 1)}</span>
            <span class="nm">{enemyName(i)}<small>{t(ELEMENT_NAME[e.element])}</small></span>
            {#if e.max_toughness}<span class="tough">{#each Array(e.max_toughness) as _, k (k)}<i class:on={k < e.toughness}></i>{/each}</span>{/if}
            <span class="bar hp"><i style:width={pct(e.hp, e.max_hp)}></i><s style:width={pct(e.shield, e.max_hp)}></s></span>
            <span class="bar gauge"><i style:width={pct(e.gauge, 1000)}></i></span>
            <span class="sts">{#each e.statuses as s (s.kind)}<em class:bad={!isBuff(s.kind)}>{t(SHORT[s.kind])}{s.turns}</em>{/each}</span>
            {#each pops.filter((p) => p.side === 'enemy' && p.slot === i) as p (p.key)}<b class="pop {p.kind}">{p.text}</b>{/each}
          </button>
        {/each}
      </div>
    </section>

    <section class="command panel">
      {#if auto}
        <p class="hint">{t('自動戰鬥中…')}</p>
      {:else if !actor}
        <p class="hint">{playing ? '' : t('等待中…')}</p>
      {:else}
        <div class="who el-{actor.element}">
          <b>{allyName(current)}</b>
          <span class="cheer" title={t('聲援：每次我方出手 +1，花 {n} 格可以接棒', { n: view.relay_cost })}>
            {#each Array(view.cheer_max) as _, k (k)}<i class:on={k < view.cheer}></i>{/each}
          </span>
          <button type="button" class="relay" class:on={sel.relayMode || sel.relay !== null} disabled={view.cheer < view.relay_cost} onclick={toggleRelay}>
            {sel.relay !== null ? t('接棒：{name}', { name: allyName(sel.relay) }) : sel.relayMode ? t('點一位隊友') : t('接棒')}
          </button>
        </div>
        <div class="skills">
          {#each [0, 1, 2] as i (i)}
            {@const s = skillOf(current, i)}
            {@const cd = actor.cooldowns[i]}
            <button type="button" class:on={sel.skill === i} disabled={!usable(i)} onclick={() => pickSkill(i)}>
              <b>S{i + 1}・{t(s.name)}</b>
              <small>{cd > 0 ? t('冷卻中（{n}）', { n: cd }) : cooldownText(s)}</small>
            </button>
          {/each}
        </div>
        {#if skill}<p class="desc">{skillText(skill)}</p>{/if}
        <button class="btn go" type="button" disabled={problem !== ''} onclick={() => send(command())}>{t('出手')}</button>
      {/if}
      <ol class="log" aria-live="polite">
        {#each log as line, i (i + line)}<li>{line}</li>{/each}
      </ol>
    </section>

    {#if banner}
      <div class="banner" class:foe={banner.enemy}>
        {#if banner.portrait}<img src={portrait(banner.portrait, 'happy')} alt="" draggable="false" />{/if}
        <span>{banner.text}</span>
      </div>
    {/if}

    {#if result}
      <div class="scrim" role="dialog" aria-modal="true">
        <div class="panel res">
          <h3>{result.won ? t('勝利！') : t('敗北…')}</h3>
          {#if result.pending}
            <p>{t('伺服器驗證中…')}</p>
          {:else if result.failed}
            <p>{session.error || t('結算失敗')}</p>
          {:else}
            <ul class="stars">
              <li class:on={stars[0]}>★ {t('通關')}</li>
              <li class:on={stars[1]}>★ {t('{n} 回合內', { n: stage.star_turns })}</li>
              <li class:on={stars[2]}>★ {t('沒有人倒下')}</li>
            </ul>
            {#if result.won}
              <p>{t('我方 {n} 回合過關', { n: result.outcome.rounds })}</p>
              <p>{t('金幣 +{g}・練習筆記 +{n}', { g: result.rewards.gold, n: result.rewards.notes })}</p>
              {#if result.rewards.first}<p class="first">{t('首次通關：星屑 +{s}', { s: result.rewards.stardust })}</p>{/if}
            {:else}
              <p>{t('練等級、換隊伍或調整出手順序再來一次吧。')}</p>
            {/if}
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

  .info {
    color: var(--gold);
    font-weight: 700;
    display: flex;
    gap: 2cqh;
    align-items: center;
  }

  .env {
    font-size: 0.75em;
    color: var(--ivory);
    background: rgba(255, 255, 255, 0.12);
    border-radius: 0.6cqh;
    padding: 0.3cqh 1.2cqh;
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

  /* ---- 時間軸 ---- */
  .timeline {
    position: absolute;
    top: 8.5cqh;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 1cqh;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tl {
    position: relative;
    width: 7cqh;
    height: 7cqh;
    border-radius: 50%;
    overflow: hidden;
    border: 0.35cqh solid var(--el);
    background: #111;
    display: grid;
    place-items: center;
  }

  .tl.now {
    width: 9cqh;
    height: 9cqh;
    margin-top: -1cqh;
    box-shadow: 0 0 1.6cqh var(--gold);
    border-color: var(--gold);
  }

  .tl.tl-enemy {
    border-radius: 1cqh;
    background: color-mix(in srgb, var(--el) 50%, #111);
  }

  .tl img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: 50% 10%;
  }

  .tl .tok {
    font-weight: 900;
    font-size: 1.2em;
  }

  .ch {
    position: absolute;
    right: 0;
    bottom: 0;
    font-size: 0.8em;
  }

  /* ---- 場地：我方在左、敵人在右 ---- */
  .field {
    position: absolute;
    top: 19cqh;
    left: 0;
    right: 0;
    height: 52cqh;
  }

  .allies,
  .enemies {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 76cqh;
  }

  .allies {
    left: 4cqh;
  }

  .enemies {
    right: 4cqh;
  }

  .ally,
  .enemy {
    position: absolute;
    width: 34cqh;
    height: 24cqh;
    padding: 0;
    border-radius: 1.2cqh;
    text-align: left;
    transition: transform 0.15s;
  }

  /* 前後兩排錯開 */
  .ally.pos-0 { left: 38cqh; top: 0; }
  .ally.pos-1 { left: 38cqh; top: 27cqh; }
  .ally.pos-2 { left: 0; top: 4cqh; }
  .ally.pos-3 { left: 0; top: 31cqh; }
  .enemy.pos-0 { left: 0; top: 0; }
  .enemy.pos-1 { left: 0; top: 27cqh; }
  .enemy.pos-2 { left: 38cqh; top: 4cqh; }
  .enemy.pos-3 { left: 38cqh; top: 31cqh; }
  .enemy.pos-4 { left: 19cqh; top: 14cqh; }
  .enemy.boss { left: 14cqh; top: 4cqh; width: 46cqh; height: 44cqh; }

  .ally {
    overflow: hidden;
    border: 0.4cqh solid var(--rarity);
    background: radial-gradient(circle at 30% 30%, color-mix(in srgb, var(--el) 35%, #0a2a1f) 0%, #071a13 100%);
  }

  .ally img {
    position: absolute;
    right: -3cqh;
    top: -1cqh;
    height: 30cqh;
  }

  .enemy {
    border: var(--line) solid rgba(0, 0, 0, 0.5);
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.55), rgba(0, 0, 0, 0.25));
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 0.6cqh;
    padding: 1.2cqh 1.6cqh;
  }

  .enemy .tok {
    width: 9cqh;
    height: 9cqh;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 2em;
    font-weight: 900;
    background: radial-gradient(circle at 35% 30%, color-mix(in srgb, var(--el) 40%, white), var(--el));
    margin-bottom: auto;
  }

  .enemy.boss .tok {
    width: 20cqh;
    height: 20cqh;
    font-size: 4em;
  }

  .pickable {
    cursor: pointer;
  }

  .enemy.pickable:hover,
  .ally.pickable:hover {
    transform: translateY(-0.6cqh);
  }

  .acting {
    box-shadow: 0 0 2.4cqh var(--gold);
  }

  .ally.acting {
    border-color: var(--gold);
  }

  .target {
    outline: 0.5cqh solid var(--accent);
  }

  .relay {
    outline: 0.5cqh dashed var(--shield);
  }

  .down {
    opacity: 0.35;
    filter: grayscale(1);
  }

  .el {
    position: absolute;
    top: 1cqh;
    left: 1cqh;
    width: 4cqh;
    height: 4cqh;
    border-radius: 0.6cqh;
    display: grid;
    place-items: center;
    font-weight: 900;
    background: var(--el);
  }

  .nm {
    display: flex;
    flex-direction: column;
    font-weight: 800;
    text-shadow: 0 0.3cqh 0.4cqh #000;
    font-size: 0.85em;
  }

  .ally .nm {
    position: absolute;
    left: 1.2cqh;
    bottom: 7cqh;
  }

  .nm small {
    font-size: 0.65em;
    color: var(--gold);
  }

  .enemy .nm {
    align-items: center;
  }

  .bars {
    position: absolute;
    left: 1.2cqh;
    right: 1.2cqh;
    bottom: 3.4cqh;
    display: flex;
    flex-direction: column;
    gap: 0.4cqh;
  }

  .bar {
    position: relative;
    width: 100%;
    height: 1.3cqh;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 1cqh;
    overflow: hidden;
  }

  .bar.gauge {
    height: 0.7cqh;
  }

  .bar i,
  .bar s {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    transition: width 0.25s;
  }

  .bar.hp i {
    background: var(--hp);
  }

  .bar.gauge i {
    background: var(--gold);
  }

  .bar s {
    background: var(--shield);
    opacity: 0.75;
  }

  .hpnum {
    position: absolute;
    left: 1.2cqh;
    bottom: 0.6cqh;
    font-size: 0.6em;
    font-variant-numeric: tabular-nums;
  }

  .sts {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4cqh;
    min-height: 2.4cqh;
  }

  .ally .sts {
    position: absolute;
    right: 1cqh;
    bottom: 0.4cqh;
    justify-content: flex-end;
  }

  .sts em {
    font-style: normal;
    font-size: 0.55em;
    background: rgba(40, 120, 220, 0.85);
    border-radius: 0.4cqh;
    padding: 0 0.6cqh;
  }

  .sts em.bad {
    background: rgba(200, 60, 40, 0.9);
  }

  .tag,
  .charge {
    position: absolute;
    top: 1cqh;
    right: 1cqh;
    background: var(--accent);
    border: 0.25cqh solid var(--gold);
    border-radius: 0.8cqh;
    padding: 0.2cqh 1cqh;
    font-weight: 800;
    font-size: 0.7em;
  }

  .charge {
    animation: blink 0.7s infinite alternate;
  }

  @keyframes blink {
    to {
      filter: brightness(1.6);
    }
  }

  .tough {
    display: flex;
    gap: 0.4cqh;
  }

  .tough i {
    width: 1.6cqh;
    height: 1.6cqh;
    transform: rotate(45deg);
    border: 0.2cqh solid var(--shield);
  }

  .tough i.on {
    background: var(--shield);
  }

  .pop {
    position: absolute;
    left: 50%;
    top: 35%;
    transform: translateX(-50%);
    font-size: 1.3em;
    font-weight: 900;
    white-space: nowrap;
    pointer-events: none;
    text-shadow: 0 0.3cqh 0 #000;
    animation: rise 1.1s ease-out forwards;
    z-index: 2;
  }

  .pop.dmg { color: #ffd2c7; }
  .pop.crit { color: #ffcf4a; font-size: 1.6em; }
  .pop.graze { color: #c8c8c8; }
  .pop.heal { color: var(--hp); }
  .pop.buff { color: var(--shield); }
  .pop.debuff { color: #ff9f6b; }

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

  /* ---- 指令 ---- */
  .command {
    position: absolute;
    left: 3cqh;
    right: 3cqh;
    bottom: 2.4cqh;
    height: 24cqh;
    padding: 1.4cqh 2cqh;
    font-size: 0.85em;
    display: grid;
    grid-template-columns: 1fr 50cqh;
    grid-template-rows: auto auto 1fr;
    column-gap: 2cqh;
  }

  .hint {
    margin: 0;
    grid-column: 1;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 1.6cqh;
  }

  .who b {
    font-size: 1.2em;
    border-left: 0.8cqh solid var(--el);
    padding-left: 1cqh;
  }

  .cheer {
    display: flex;
    gap: 0.6cqh;
  }

  .cheer i {
    width: 2cqh;
    height: 2cqh;
    border-radius: 50%;
    border: 0.25cqh solid var(--gold-deep);
  }

  .cheer i.on {
    background: var(--gold);
  }

  .relay {
    background: var(--ivory-deep);
    border: 0.25cqh solid var(--gold-deep);
    border-radius: 0.8cqh;
    padding: 0.3cqh 1.4cqh;
    color: var(--ink);
  }

  .relay.on {
    background: var(--shield);
  }

  .skills {
    display: flex;
    gap: 1cqh;
    margin-top: 1cqh;
    grid-column: 1;
  }

  .skills button {
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

  .skills button.on {
    background: var(--accent);
    color: var(--ivory);
    border-color: var(--gold);
  }

  .skills button:disabled {
    opacity: 0.45;
  }

  .skills small {
    font-size: 0.75em;
  }

  .desc {
    grid-column: 1;
    margin: 0.8cqh 0 0;
    font-size: 0.9em;
    color: #4a4135;
  }

  .go {
    position: absolute;
    right: 54cqh;
    bottom: 1.4cqh;
    padding: 1cqh 5cqh;
  }

  .log {
    grid-column: 2;
    grid-row: 1 / 4;
    margin: 0;
    padding: 0.8cqh 1.2cqh;
    list-style: none;
    background: rgba(15, 59, 46, 0.1);
    border-radius: 0.8cqh;
    font-size: 0.8em;
    color: #3a3326;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 0.2cqh;
    overflow: hidden;
  }

  /* ---- 演出 ---- */
  .banner {
    position: absolute;
    top: 30cqh;
    left: 0;
    right: 0;
    height: 22cqh;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3cqh;
    background: linear-gradient(90deg, transparent, rgba(210, 64, 47, 0.85) 20%, rgba(210, 64, 47, 0.85) 80%, transparent);
    transform: skewY(-4deg);
    pointer-events: none;
    animation: slide 0.3s ease-out;
    z-index: 10;
    overflow: hidden;
  }

  .banner.foe {
    background: linear-gradient(90deg, transparent, rgba(60, 30, 110, 0.9) 20%, rgba(60, 30, 110, 0.9) 80%, transparent);
  }

  .banner img {
    height: 40cqh;
    margin-top: 16cqh;
    transform: skewY(4deg);
  }

  .banner span {
    font-size: 3em;
    font-weight: 900;
    letter-spacing: 0.2em;
    color: var(--ivory);
    -webkit-text-stroke: 0.3cqh #000;
    paint-order: stroke fill;
    transform: skewY(4deg);
  }

  @keyframes slide {
    from {
      transform: skewY(-4deg) translateX(-30%);
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
    width: 64cqh;
    padding: 3cqh;
    text-align: center;
  }

  .res h3 {
    margin: 0 0 1cqh;
    font-size: 2em;
  }

  .stars {
    list-style: none;
    padding: 0;
    display: flex;
    justify-content: center;
    gap: 2cqh;
    color: #9a8f78;
  }

  .stars li.on {
    color: #d49a00;
    font-weight: 800;
  }

  .first {
    color: var(--tide);
    font-weight: 800;
  }

  .warn {
    color: var(--accent-deep);
    font-size: 0.8em;
  }
</style>
