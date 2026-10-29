<script>
  // 戰鬥畫面：引擎（wasm）在這台裝置上跑，玩家的每個指令都記下來，打完送給 Server 重算、發獎勵。
  // 操作：輪到某位隊友時選一招（S1～S3）→ 點目標 → 出手。聲援 3 格以上可以「接棒」，指定一位隊友緊接著出手。
  import { onMount, onDestroy } from 'svelte';
  import { Button, Tag, Modal, ConfirmDialog } from 'bluebolt-ui';
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

  let retreating = $state(false);

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
      <span class="wave"><b>WAVE</b>{view.wave + 1}<small>／{view.waves}</small></span>
      <span class="turns"><b>TURN</b>{view.turns}</span>
      {#if view.env}<span class="env"><Tag variant="yellow" size="sm">{t(view.env.name)}</Tag>{t(view.env.text)}</span>{/if}
      <div class="tools">
        <Button shape="rect" size="sm" variant={auto ? 'primary' : 'ghost'} onclick={toggleAuto} disabled={view.phase !== 'command'}>{t('自動')}</Button>
        <Button shape="rect" size="sm" variant={fast ? 'primary' : 'ghost'} onclick={() => (fast = !fast)}>×2</Button>
        <Button shape="rect" size="sm" variant="ink" onclick={() => (retreating = true)} disabled={view.phase !== 'command'}>{t('撤退')}</Button>
      </div>
    </header>

    <ol class="timeline" aria-label={t('出手順序')}>
      <li class="lbl">ORDER</li>
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
            class="unit ally rarity-{def.rarity} el-{a.element} pos-{i}"
            class:down={a.hp <= 0}
            class:acting={isActing('ally', i)}
            class:target={targetSide === 'ally' && sel.target === i}
            class:relay={sel.relay === i}
            class:pickable={canAct && ((targetSide === 'ally' && a.hp > 0) || (sel.relayMode && i !== current && a.hp > 0))}
            onclick={() => pickAlly(i)}
          >
            <img src={portrait(def.character, a.hp <= 0 ? 'sad' : view.phase === 'won' ? 'win' : 'normal')} alt="" draggable="false" />
            <span class="el-mark">{t(ELEMENT_NAME[a.element])}</span>
            <span class="nm">{allyName(i)}<small>{t(def.name)}</small></span>
            <span class="bars">
              <span class="bar hp"><i style:width={pct(a.hp, a.max_hp)}></i><s style:width={pct(a.shield, a.max_hp)}></s></span>
              <span class="bar gauge"><i style:width={pct(a.gauge, 1000)}></i></span>
            </span>
            <span class="hpnum">{a.hp}<small>／{a.max_hp}</small></span>
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
            class="unit enemy el-{e.element} pos-{i}"
            class:boss={e.boss}
            class:down={e.hp <= 0}
            class:acting={isActing('enemy', i)}
            class:target={targetSide === 'enemy' && sel.target === i}
            class:pickable={canAct && e.hp > 0}
            onclick={() => pickEnemy(i)}
          >
            {#if e.charge && e.hp > 0}<span class="charge">{t('蓄力 {n}', { n: e.charge.turns + 1 })}</span>{/if}
            <span class="tok">{t(ENEMIES.get(e.id).name).slice(0, 1)}</span>
            <span class="nm"><span class="el-mark">{t(ELEMENT_NAME[e.element])}</span>{enemyName(i)}</span>
            {#if e.max_toughness}<span class="tough">{#each Array(e.max_toughness) as _, k (k)}<i class:on={k < e.toughness}></i>{/each}</span>{/if}
            <span class="bar hp"><i style:width={pct(e.hp, e.max_hp)}></i><s style:width={pct(e.shield, e.max_hp)}></s></span>
            <span class="bar gauge"><i style:width={pct(e.gauge, 1000)}></i></span>
            <span class="sts">{#each e.statuses as s (s.kind)}<em class:bad={!isBuff(s.kind)}>{t(SHORT[s.kind])}{s.turns}</em>{/each}</span>
            {#each pops.filter((p) => p.side === 'enemy' && p.slot === i) as p (p.key)}<b class="pop {p.kind}">{p.text}</b>{/each}
          </button>
        {/each}
      </div>
    </section>

    <section class="command">
      <div class="main">
        {#if auto}
          <p class="hint">{t('自動戰鬥中…')}</p>
        {:else if !actor}
          <p class="hint">{playing ? '' : t('等待中…')}</p>
        {:else}
          <div class="who el-{actor.element}">
            <span class="el-mark">{t(ELEMENT_NAME[actor.element])}</span>
            <b>{allyName(current)}</b>
            <span class="cheer" title={t('聲援：每次我方出手 +1，花 {n} 格可以接棒', { n: view.relay_cost })}>
              <em>CHEER</em>
              {#each Array(view.cheer_max) as _, k (k)}<i class:on={k < view.cheer}></i>{/each}
            </span>
            <Button shape="rect" size="sm" variant={sel.relayMode || sel.relay !== null ? 'accent' : 'ghost'} disabled={view.cheer < view.relay_cost} onclick={toggleRelay}>
              {sel.relay !== null ? t('接棒：{name}', { name: allyName(sel.relay) }) : sel.relayMode ? t('點一位隊友') : t('接棒')}
            </Button>
          </div>
          <div class="skills">
            {#each [0, 1, 2] as i (i)}
              {@const s = skillOf(current, i)}
              {@const cd = actor.cooldowns[i]}
              <button type="button" class:on={sel.skill === i} disabled={!usable(i)} onclick={() => pickSkill(i)}>
                <span class="no">S{i + 1}</span>
                <b>{t(s.name)}</b>
                <small>{cd > 0 ? t('冷卻中（{n}）', { n: cd }) : cooldownText(s)}</small>
              </button>
            {/each}
            <div class="go">
              <Button size="lg" en="Action" disabled={problem !== ''} onclick={() => send(command())}>{t('出手')}</Button>
            </div>
          </div>
          {#if skill}<p class="desc">{skillText(skill)}</p>{/if}
        {/if}
      </div>
      <ol class="log" aria-live="polite">
        {#each log as line, i (i + line)}<li>{line}</li>{/each}
      </ol>
    </section>

    {#if banner}
      <div class="banner" class:foe={banner.enemy}>
        <span class="band"></span>
        {#if banner.portrait}<img src={portrait(banner.portrait, 'happy')} alt="" draggable="false" />{/if}
        <span class="text">{banner.text}</span>
      </div>
    {/if}

    <ConfirmDialog bind:open={retreating} title={t('撤退')} en="Retreat" danger message={t('撤退的話這場不會有獎勵，確定嗎？')} confirmLabel={t('撤退')} onconfirm={onexit} />

    <Modal open={!!result} title={result?.won ? t('勝利！') : t('敗北…')} sub={result?.won ? 'Victory' : 'Defeat'} width="560px" dismissable={false}>
      {#if result}
        {#if result.pending}
          <p>{t('伺服器驗證中…')}</p>
        {:else if result.failed}
          <p>{t('結算失敗')}</p>
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
      {/if}
      {#snippet footer()}
        <Button shape="rect" disabled={result?.pending} onclick={onexit}>{t('返回')}</Button>
      {/snippet}
    </Modal>
  </div>
{/if}

<style>
  .battle {
    position: absolute;
    inset: 0;
  }

  /* ---- 上方：波次、回合、環境、工具 ---- */
  header {
    position: absolute;
    top: 14px;
    left: 24px;
    right: 24px;
    height: 44px;
    display: flex;
    align-items: center;
    gap: 18px;
    z-index: 3;
  }

  .wave,
  .turns {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 2px 16px;
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    font: italic 400 26px var(--bb-font-display);
    clip-path: polygon(10px 0, 100% 0, calc(100% - 10px) 100%, 0 100%);
  }

  .wave b,
  .turns b {
    font: italic 700 11px var(--bb-font-cond);
    letter-spacing: 0.2em;
    color: var(--bb-yellow);
  }

  .wave small {
    font-size: 16px;
    color: var(--bb-on-ink-2);
  }

  .env {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 700;
  }

  .tools {
    margin-left: auto;
    display: flex;
    gap: 6px;
  }

  /* ---- 時間軸 ---- */
  .timeline {
    position: absolute;
    top: 68px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 6px 14px 6px 10px;
    list-style: none;
    background: var(--bb-strip);
    clip-path: polygon(14px 0, 100% 0, calc(100% - 14px) 100%, 0 100%);
    z-index: 3;
  }

  .lbl {
    font: italic 700 11px var(--bb-font-cond);
    letter-spacing: 0.2em;
    color: var(--bb-yellow);
    padding: 0 8px 0 10px;
  }

  .tl {
    position: relative;
    width: 44px;
    height: 44px;
    overflow: hidden;
    background: #1b2235;
    box-shadow: inset 0 -3px 0 var(--el);
    display: grid;
    place-items: center;
    clip-path: polygon(6px 0, 100% 0, calc(100% - 6px) 100%, 0 100%);
  }

  .tl.now {
    width: 58px;
    height: 58px;
    box-shadow: inset 0 0 0 3px var(--bb-yellow);
  }

  .tl.tl-enemy {
    background: color-mix(in srgb, var(--el) 45%, #1b2235);
  }

  .tl img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: 50% 10%;
  }

  .tl .tok {
    color: #fff;
    font-weight: 900;
    font-size: 20px;
  }

  .ch {
    position: absolute;
    right: 2px;
    bottom: 0;
    font-size: 13px;
  }

  /* ---- 場地：我方在左、敵人在右 ---- */
  .field {
    position: absolute;
    top: 136px;
    left: 0;
    right: 0;
    height: 380px;
  }

  .allies,
  .enemies {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 560px;
  }

  .allies {
    left: 30px;
  }

  .enemies {
    right: 30px;
  }

  .unit {
    position: absolute;
    width: 250px;
    height: 168px;
    padding: 0;
    border: 0;
    text-align: left;
    color: #fff;
    transition: transform var(--bb-dur-2) var(--bb-ease);
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 14px), calc(100% - 14px) 100%, 0 100%);
  }

  /* 前後兩排錯開 */
  .ally.pos-0 { left: 290px; top: 0; }
  .ally.pos-1 { left: 290px; top: 186px; }
  .ally.pos-2 { left: 0; top: 24px; }
  .ally.pos-3 { left: 0; top: 210px; }
  .enemy.pos-0 { left: 0; top: 0; }
  .enemy.pos-1 { left: 0; top: 186px; }
  .enemy.pos-2 { left: 290px; top: 24px; }
  .enemy.pos-3 { left: 290px; top: 210px; }
  .enemy.pos-4 { left: 145px; top: 105px; }
  .enemy.boss { left: 90px; top: 6px; width: 380px; height: 366px; }

  .ally {
    overflow: hidden;
    background: linear-gradient(115deg, color-mix(in srgb, var(--el) 55%, #0b0f1a) 0 34%, #141a2c 34%);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18), inset 0 -3px 0 var(--rarity);
  }

  .ally img {
    position: absolute;
    right: -24px;
    top: -6px;
    height: 220px;
  }

  .enemy {
    background:
      repeating-linear-gradient(135deg, rgb(255 255 255 / 0.04) 0 6px, transparent 6px 12px),
      linear-gradient(180deg, #2a1730, #120b18);
    box-shadow: inset 0 0 0 1px rgb(255 80 100 / 0.35);
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: flex-end;
    gap: 5px;
    padding: 10px 14px 12px;
  }

  .enemy .tok {
    align-self: center;
    width: 64px;
    height: 64px;
    margin-bottom: auto;
    display: grid;
    place-items: center;
    font-size: 30px;
    font-weight: 900;
    background: radial-gradient(circle at 35% 30%, color-mix(in srgb, var(--el) 40%, white), var(--el));
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  }

  .enemy.boss .tok {
    width: 170px;
    height: 170px;
    font-size: 80px;
  }

  .pickable {
    cursor: pointer;
  }

  .unit.pickable:hover {
    transform: translateY(-4px);
  }

  .acting {
    box-shadow: inset 0 0 0 3px var(--bb-yellow), 0 0 0 0 transparent;
  }

  .acting::after {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    width: 60px;
    height: 6px;
    background: var(--bb-yellow);
  }

  .target {
    box-shadow: inset 0 0 0 3px #7fb6ff, inset 0 0 24px rgb(61 155 255 / 0.55);
  }

  .relay {
    box-shadow: inset 0 0 0 3px var(--bb-yellow), inset 0 0 24px rgb(255 210 31 / 0.4);
  }

  .down {
    opacity: 0.35;
    filter: grayscale(1);
  }

  .ally > .el-mark {
    position: absolute;
    top: 8px;
    left: 8px;
  }

  .nm {
    display: flex;
    flex-direction: column;
    font-weight: 900;
    font-size: 16px;
    text-shadow: 0 2px 4px #000;
    z-index: 1;
  }

  .ally .nm {
    position: absolute;
    left: 12px;
    bottom: 56px;
  }

  .nm small {
    font-size: 11px;
    color: var(--bb-yellow);
  }

  .enemy .nm {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    font-size: 14px;
  }

  .enemy .nm .el-mark {
    width: 20px;
    height: 20px;
    font-size: 10px;
  }

  .bars {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 26px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 1;
  }

  .bar {
    position: relative;
    display: block;
    width: 100%;
    height: 10px;
    background: rgb(0 0 0 / 0.65);
    transform: skewX(var(--bb-skew));
    overflow: hidden;
  }

  .bar.gauge {
    height: 5px;
  }

  .bar i,
  .bar s {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    transition: width 0.25s var(--bb-ease);
  }

  .bar.hp i {
    background: var(--hp);
  }

  .enemy .bar.hp i {
    background: var(--hp-foe);
  }

  .bar.gauge i {
    background: var(--gauge);
  }

  .bar s {
    background: var(--shield);
    opacity: 0.8;
  }

  .hpnum {
    position: absolute;
    left: 12px;
    bottom: 6px;
    font: italic 400 16px var(--bb-font-display);
    z-index: 1;
  }

  .hpnum small {
    font-size: 11px;
    opacity: 0.7;
  }

  .sts {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    min-height: 16px;
    z-index: 1;
  }

  .ally .sts {
    position: absolute;
    right: 8px;
    bottom: 6px;
    justify-content: flex-end;
    max-width: 150px;
  }

  .sts em {
    font-style: normal;
    font-size: 10px;
    font-weight: 800;
    background: var(--bb-blue);
    padding: 0 5px;
  }

  .sts em.bad {
    background: #c8263a;
  }

  .tag,
  .charge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: var(--bb-yellow);
    color: var(--bb-on-yellow);
    padding: 2px 10px;
    font-weight: 900;
    font-size: 12px;
    transform: skewX(var(--bb-skew));
    z-index: 2;
  }

  .charge {
    animation: blink 0.6s infinite alternate;
  }

  @keyframes blink {
    to {
      background: #fff;
    }
  }

  .tough {
    display: flex;
    gap: 4px;
  }

  .tough i {
    width: 10px;
    height: 10px;
    transform: rotate(45deg);
    border: 1.5px solid var(--shield);
  }

  .tough i.on {
    background: var(--shield);
  }

  .pop {
    position: absolute;
    left: 50%;
    top: 34%;
    transform: translateX(-50%);
    font: italic 400 30px var(--bb-font-display);
    white-space: nowrap;
    pointer-events: none;
    -webkit-text-stroke: 1.5px #000;
    paint-order: stroke fill;
    animation: rise 1.1s var(--bb-ease) forwards;
    z-index: 4;
  }

  .pop.dmg { color: #fff; }
  .pop.crit { color: var(--bb-yellow); font-size: 40px; }
  .pop.graze { color: #c8c8c8; font-size: 22px; }
  .pop.heal { color: var(--hp); }
  .pop.buff { color: #8fd0ff; font: 900 20px var(--bb-font-zh); }
  .pop.debuff { color: #ff9a7a; font: 900 20px var(--bb-font-zh); }

  @keyframes rise {
    from {
      opacity: 1;
      margin-top: 0;
    }
    to {
      opacity: 0;
      margin-top: -50px;
    }
  }

  /* ---- 指令 ---- */
  .command {
    position: absolute;
    left: 24px;
    right: 24px;
    bottom: 18px;
    height: 170px;
    display: flex;
    gap: 12px;
  }

  .main {
    flex: 1;
    padding: 12px 18px;
    background: rgb(11 15 26 / 0.9);
    color: var(--bb-on-ink);
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 14px), calc(100% - 14px) 100%, 0 100%);
    box-shadow: inset 0 3px 0 var(--bb-blue);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hint {
    margin: 0;
    color: var(--bb-on-ink-2);
  }

  .who {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .who b {
    font-size: 20px;
    font-weight: 900;
  }

  .cheer {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 12px;
  }

  .cheer em {
    font: italic 700 11px var(--bb-font-cond);
    letter-spacing: 0.2em;
    color: var(--bb-yellow);
    margin-right: 4px;
  }

  .cheer i {
    width: 16px;
    height: 16px;
    border: 1.5px solid var(--bb-yellow);
    transform: rotate(45deg) scale(0.8);
  }

  .cheer i.on {
    background: var(--bb-yellow);
  }

  .skills {
    display: flex;
    gap: 10px;
    align-items: stretch;
  }

  .skills > button {
    flex: 1;
    max-width: 230px;
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 10px;
    align-items: center;
    padding: 8px 14px;
    border: 0;
    background: var(--bb-face-gray);
    color: #fff;
    text-align: left;
    clip-path: polygon(10px 0, 100% 0, calc(100% - 10px) 100%, 0 100%);
    transition: transform var(--bb-dur-2) var(--bb-ease);
  }

  .skills > button:not(:disabled):hover {
    transform: translateY(-2px);
  }

  .skills > button.on {
    background: var(--bb-face-blue);
    box-shadow: inset 0 -3px 0 var(--bb-yellow);
  }

  .skills > button:disabled {
    opacity: 0.4;
  }

  .no {
    grid-row: 1 / 3;
    font: italic 400 30px var(--bb-font-display);
    color: var(--bb-yellow);
  }

  .skills b {
    font-size: 15px;
  }

  .skills small {
    font-size: 11px;
    opacity: 0.8;
  }

  .go {
    margin-left: auto;
    align-self: center;
  }

  .desc {
    margin: 0;
    font-size: 13px;
    color: var(--bb-on-ink-2);
  }

  .log {
    width: 340px;
    margin: 0;
    padding: 10px 14px;
    list-style: none;
    background: var(--bb-glass-strong);
    box-shadow: inset 0 0 0 1px var(--bb-line);
    font-size: 12px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 2px;
    overflow: hidden;
  }

  .log li {
    border-left: 2px solid var(--bb-blue);
    padding-left: 6px;
  }

  /* ---- 大招特寫 ---- */
  .banner {
    position: absolute;
    left: 0;
    right: 0;
    top: 230px;
    height: 170px;
    pointer-events: none;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 30px;
    overflow: hidden;
  }

  .band {
    position: absolute;
    inset: 0;
    background:
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.08) 0 2px, transparent 2px 22px),
      var(--bb-face-blue);
    border-top: 6px solid var(--bb-yellow);
    border-bottom: 6px solid var(--bb-ink);
    transform: skewY(-3deg);
    animation: band 0.32s var(--bb-ease);
  }

  .banner.foe .band {
    background:
      repeating-linear-gradient(90deg, rgb(255 255 255 / 0.06) 0 2px, transparent 2px 22px),
      linear-gradient(180deg, #4a1530, #1b0a14);
    border-top-color: #ff4d5e;
  }

  @keyframes band {
    from {
      transform: skewY(-3deg) scaleX(0);
    }
  }

  .banner img {
    position: relative;
    height: 330px;
    margin-top: 150px;
    animation: slide 0.4s var(--bb-ease);
  }

  .text {
    position: relative;
    font-size: 60px;
    font-weight: 900;
    letter-spacing: 0.2em;
    color: #fff;
    -webkit-text-stroke: 2px var(--bb-ink);
    paint-order: stroke fill;
    font-style: italic;
    animation: slide 0.4s var(--bb-ease) 0.05s both;
  }

  @keyframes slide {
    from {
      transform: translateX(-120px);
      opacity: 0;
    }
  }

  .stars {
    list-style: none;
    padding: 0;
    margin: 0 0 12px;
    display: flex;
    gap: 16px;
    color: var(--bb-ink-4);
    font-weight: 700;
  }

  .stars li.on {
    color: #c79a00;
  }

  .first {
    color: var(--bb-blue-text);
    font-weight: 800;
  }

  .warn {
    color: #c8263a;
    font-size: 13px;
  }
</style>
