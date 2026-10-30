<script>
  // 角色：左邊大圖（有戰鬥全身圖就用全身）、右邊分頁（能力／技能／養成）、下面一排手上的角色。
  // 花費與上限 Server 會再檢查一次，這裡只是先算給玩家看。
  import { Button, Segmented, Tag } from 'bluebolt-ui';
  import { session, levelUp, ascend } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, SKILLS, ELEMENT_NAME, ROLE_NAME, characterOf, unitStats, levelCap, levelCost, ascendCost, skillText, cooldownText, passiveText } from '../data.js';
  import { costumeArt, spriteArt } from '../art.js';
  import TopBar from './TopBar.svelte';
  import UnitCard from './UnitCard.svelte';

  let { onback } = $props();

  const owned = $derived(
    [...(session.profile?.costumes ?? [])].sort(
      (a, b) => COSTUMES.get(b.id).rarity - COSTUMES.get(a.id).rarity || b.level - a.level || a.id.localeCompare(b.id),
    ),
  );
  let picked = $state(null);
  let tab = $state('stats');
  const unit = $derived(owned.find((c) => c.id === picked) ?? owned[0]);
  const def = $derived(unit && COSTUMES.get(unit.id));
  const ch = $derived(unit && characterOf(unit.id));
  const bond = $derived(unit ? (session.profile.bonds[ch.id] ?? { exp: 0, level: 0 }) : null);
  const cap = $derived(unit ? levelCap(unit.ascension) : 0);
  const stats = $derived(unit ? unitStats(unit.id, unit.level, unit.ascension, bond.level) : null);
  const full = $derived(unit && spriteArt(unit.id));

  // 能力條的滿格（大約是滿等滿突破的上限）
  const BAR = [
    ['hp', 'HP', 3000],
    ['atk', '攻擊', 450],
    ['def', '防禦', 260],
    ['spd', '速度', 140],
  ];
  const RATES = [
    ['crit', '暴擊', '%'],
    ['crit_dmg', '暴擊傷害', '%'],
    ['hit', '效果命中', '%'],
    ['resist', '效果抵抗', '%'],
  ];

  /** 錢夠的話最多能升幾級（不超過上限） */
  function affordable(n) {
    const to = Math.min(cap, unit.level + n);
    const c = levelCost(unit.level, to);
    return { to, ...c, ok: to > unit.level && c.gold <= session.profile.gold && c.notes <= session.profile.notes };
  }
  const steps = $derived(unit ? [1, 5, 10, cap - unit.level].filter((n, i, a) => n > 0 && a.indexOf(n) === i).map((n) => ({ n, ...affordable(n) })) : []);
  const canAscend = $derived(unit && unit.ascension < 5 && unit.dupes > 0 && unit.level >= cap && session.profile.gold >= ascendCost(unit.ascension));

  const run = (fn) => fn().catch(() => {});
  const KIND = ['S1', 'S2', 'S3'];
</script>

<TopBar title={t('角色')} en="Roster" num="03" {onback} />

{#if unit}
  <section class="hero el-{def.element} rarity-{def.rarity}">
    <span class="slash" aria-hidden="true"></span>
    <span class="bigname" aria-hidden="true">{ch.id.toUpperCase()}</span>
    {#key unit.id}
      <img class="art" class:full src={full ?? costumeArt(unit.id, 'happy')} alt="" draggable="false" />
    {/key}
    <div class="plate">
      <span class="stars">{'★'.repeat(def.rarity)}</span>
      <h3>{t(ch.name)}</h3>
      <p class="ver">{t(def.name)}</p>
      <p class="badges">
        <span class="el-mark">{t(ELEMENT_NAME[def.element])}</span>
        <Tag variant="blue" size="sm">{t(ROLE_NAME[def.role])}</Tag>
      </p>
    </div>
    <div class="lv">
      <span>Lv.</span><b>{unit.level}</b><small>／{cap}</small>
      <span class="lvbar"><i style:width="{(unit.level / cap) * 100}%"></i></span>
    </div>
  </section>

  <section class="info">
    <Segmented
      options={[
        { value: 'stats', label: t('能力') },
        { value: 'skills', label: t('技能') },
        { value: 'grow', label: t('養成') },
      ]}
      bind:value={tab}
    />

    {#if tab === 'stats'}
      <div class="pane">
        {#each BAR as [k, label, max] (k)}
          <div class="stat">
            <span class="lbl">{t(label)}</span>
            <span class="bar"><i style:width="{Math.min(100, (stats[k] / max) * 100)}%"></i></span>
            <b>{stats[k]}</b>
          </div>
        {/each}
        <div class="rates">
          {#each RATES as [k, label, unitMark] (k)}
            <div><span>{t(label)}</span><b>{stats[k]}<small>{unitMark}</small></b></div>
          {/each}
        </div>
        <p class="intro">{t(ch.intro)}</p>
      </div>
    {:else if tab === 'skills'}
      <ul class="pane skills">
        {#each def.skills as id, i (id)}
          {@const s = SKILLS.get(id)}
          <li class="k{i}">
            <span class="no">{KIND[i]}</span>
            <div>
              <b>{t(s.name)}</b>
              <span>{skillText(s)}</span>
            </div>
            <em>{cooldownText(s)}</em>
          </li>
        {/each}
        {#each def.passives ?? [] as p (p.name)}
          <li class="kp">
            <span class="no">P</span>
            <div>
              <b>{t(p.name)}</b>
              <span>{passiveText(p)}</span>
            </div>
            <em>{t('被動')}</em>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="pane grow">
        <h4>{t('升級')}<small>{t('練習筆記＋金幣')}</small></h4>
        <div class="actions">
          {#each steps as s (s.n)}
            <Button shape="rect" size="sm" disabled={!s.ok || session.busy} onclick={() => run(() => levelUp(unit.id, s.to - unit.level))}>
              Lv {s.to}<small>● {s.gold}・▤ {s.notes}</small>
            </Button>
          {:else}
            <p class="muted">{t('已經是等級上限，突破後可以再升。')}</p>
          {/each}
        </div>
        <h4>{t('突破')}<small>{t('存 {n} 次', { n: unit.dupes })}</small></h4>
        <div class="asc">
          {#each [0, 1, 2, 3, 4] as k (k)}<i class:on={k < unit.ascension}></i>{/each}
          <Button shape="rect" size="sm" variant="secondary" disabled={!canAscend || session.busy} onclick={() => run(() => ascend(unit.id))}>
            {t('突破')}<small>● {ascendCost(unit.ascension)}</small>
          </Button>
        </div>
        <h4>{t('羈絆')}<small>{t(ch.name)}・Lv {bond.level}</small></h4>
        <span class="bondbar"><i style:width="{(bond.exp % 100)}%"></i></span>
        <p class="muted">{t('突破要先練到等級上限，並且再抽到同一個角色。同一位角色的所有版本共用羈絆：練任何一個都會加，羈絆每一級全能力 +1%。')}</p>
      </div>
    {/if}
  </section>
{/if}

<section class="strip">
  {#each owned as c (c.id)}
    <UnitCard costume={c.id} level={c.level} selected={unit?.id === c.id} width={84} onclick={() => (picked = c.id)} />
  {/each}
</section>

<style>
  /* ---- 左：大圖 ---- */
  .hero {
    position: absolute;
    left: 0;
    top: 100px;
    width: 600px;
    bottom: 150px;
    overflow: hidden;
  }

  .slash {
    position: absolute;
    left: -60px;
    top: 30px;
    width: 520px;
    bottom: 0;
    background: linear-gradient(160deg, color-mix(in srgb, var(--el) 80%, white), color-mix(in srgb, var(--el) 70%, #0b0f1a));
    transform: skewX(var(--bb-skew));
    box-shadow: 14px 0 0 var(--bb-ink);
  }

  .bigname {
    position: absolute;
    left: 20px;
    top: 20px;
    font: italic 400 110px/0.9 var(--bb-font-display);
    color: rgb(255 255 255 / 0.25);
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .art {
    position: absolute;
    left: 40px;
    bottom: -30px;
    height: 460px;
    filter: drop-shadow(8px 10px 0 rgb(11 15 26 / 0.35));
    animation: art-in var(--bb-dur-4) var(--bb-ease) both;
  }

  .art.full {
    bottom: 0;
    height: 450px;
  }

  @keyframes art-in {
    from {
      transform: translateX(calc(-60px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .plate {
    position: absolute;
    right: 24px;
    top: 40px;
    text-align: right;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .stars {
    color: var(--rarity);
    font-size: 20px;
    letter-spacing: 2px;
    text-shadow: 0 1px 0 var(--bb-ink);
  }

  h3 {
    margin: 0;
    font-size: 48px;
    font-weight: 900;
    line-height: 1.05;
    color: var(--bb-ink);
    -webkit-text-stroke: 1px #fff;
  }

  .ver {
    margin: 2px 0 8px;
    padding: 2px 14px;
    background: var(--bb-strip);
    color: var(--bb-yellow);
    font-weight: 800;
    transform: skewX(var(--bb-skew));
  }

  .badges {
    display: flex;
    gap: 6px;
    margin: 0;
  }

  .lv {
    position: absolute;
    right: 24px;
    bottom: 16px;
    display: grid;
    grid-template-columns: auto auto auto;
    align-items: baseline;
    gap: 4px;
    padding: 8px 18px 10px;
    background: var(--bb-strip);
    color: #fff;
    clip-path: polygon(12px 0, 100% 0, calc(100% - 12px) 100%, 0 100%);
  }

  .lv b {
    font: italic 400 46px/1 var(--bb-font-display);
  }

  .lv small {
    color: var(--bb-on-ink-2);
  }

  .lvbar {
    grid-column: 1 / 4;
    height: 5px;
    background: rgb(255 255 255 / 0.2);
  }

  .lvbar i {
    display: block;
    height: 100%;
    background: var(--bb-yellow);
  }

  /* ---- 右：分頁 ---- */
  .info {
    position: absolute;
    left: 640px;
    right: 40px;
    top: 112px;
    bottom: 160px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .pane {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 20px;
    background: rgb(11 15 26 / 0.9);
    color: var(--bb-on-ink);
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 16px), calc(100% - 16px) 100%, 0 100%);
    box-shadow: inset 0 3px 0 var(--bb-blue);
    margin: 0;
  }

  .stat {
    display: grid;
    grid-template-columns: 70px 1fr 70px;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .lbl {
    font-weight: 800;
    color: var(--bb-on-ink-2);
  }

  .stat .bar {
    height: 12px;
    background: rgb(255 255 255 / 0.1);
    transform: skewX(var(--bb-skew));
  }

  .stat .bar i {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, var(--bb-blue-2), var(--bb-sky));
    transition: width var(--bb-dur-4) var(--bb-ease);
  }

  .stat b {
    font: italic 400 28px/1 var(--bb-font-display);
    text-align: right;
  }

  .rates {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin-top: 16px;
  }

  .rates div {
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
    background: rgb(255 255 255 / 0.06);
    border-left: 3px solid var(--bb-yellow);
  }

  .rates span {
    font-size: 11px;
    color: var(--bb-on-ink-2);
  }

  .rates b {
    font: italic 400 26px var(--bb-font-display);
  }

  .rates small {
    font-size: 14px;
  }

  .intro {
    margin: 16px 0 0;
    color: var(--bb-on-ink-2);
    font-size: 14px;
  }

  .skills {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .skills li {
    display: grid;
    grid-template-columns: 56px 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 8px 0;
    background: rgb(255 255 255 / 0.05);
  }

  .no {
    height: 100%;
    min-height: 52px;
    display: grid;
    place-items: center;
    font: italic 400 26px var(--bb-font-display);
    background: var(--bb-face-gray);
    clip-path: polygon(0 0, 100% 0, calc(100% - 10px) 100%, 0 100%);
  }

  .k1 .no {
    background: var(--bb-face-blue);
  }

  .k2 .no {
    background: var(--bb-face-yellow);
    color: var(--bb-on-yellow);
  }

  .kp .no {
    background: linear-gradient(180deg, #8a4fd6, #4a1f8a);
  }

  .skills b {
    display: block;
    font-size: 17px;
  }

  .skills div span {
    font-size: 13px;
    color: var(--bb-on-ink-2);
  }

  .skills em {
    font-style: normal;
    font-size: 11px;
    font-weight: 800;
    padding: 2px 8px;
    background: rgb(255 255 255 / 0.12);
    white-space: nowrap;
  }

  .grow h4 {
    margin: 4px 0 8px;
    display: flex;
    align-items: baseline;
    gap: 10px;
    font-size: 17px;
  }

  .grow h4 small {
    font-size: 12px;
    font-weight: 400;
    color: var(--bb-on-ink-2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 14px;
  }

  .actions small,
  .asc small {
    margin-left: 8px;
    font-size: 11px;
    font-weight: 400;
    opacity: 0.85;
  }

  .asc {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }

  .asc i {
    width: 20px;
    height: 20px;
    transform: rotate(45deg) scale(0.8);
    border: 2px solid var(--bb-yellow);
  }

  .asc i.on {
    background: var(--bb-yellow);
  }

  .bondbar {
    display: block;
    height: 10px;
    background: rgb(255 255 255 / 0.1);
    margin-bottom: 10px;
  }

  .bondbar i {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, #ff7eb6, #ffb3d1);
  }

  .muted {
    margin: 0;
    font-size: 12px;
    color: var(--bb-on-ink-2);
  }

  /* ---- 下：角色列 ---- */
  .strip {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 140px;
    padding: 12px 40px;
    display: flex;
    gap: 10px;
    overflow-x: auto;
    background: linear-gradient(0deg, rgb(11 15 26 / 0.85), rgb(11 15 26 / 0.6));
  }
</style>
