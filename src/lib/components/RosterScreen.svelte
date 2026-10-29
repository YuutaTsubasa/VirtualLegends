<script>
  // 角色一覽與養成：升級、突破。花費與上限 Server 會再檢查一次，這裡只是先算給玩家看。
  import { Panel, Button, StatList, Tag, SectionBar } from 'bluebolt-ui';
  import { session, levelUp, ascend } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, SKILLS, ELEMENT_NAME, ROLE_NAME, characterOf, unitStats, levelCap, levelCost, ascendCost, skillText, cooldownText, passiveText } from '../data.js';
  import { portrait } from '../characters.js';
  import TopBar from './TopBar.svelte';
  import UnitCard from './UnitCard.svelte';

  let { onback } = $props();

  const owned = $derived(
    [...(session.profile?.costumes ?? [])].sort(
      (a, b) => COSTUMES.get(b.id).rarity - COSTUMES.get(a.id).rarity || b.level - a.level || a.id.localeCompare(b.id),
    ),
  );
  let picked = $state(null);
  const unit = $derived(owned.find((c) => c.id === picked) ?? owned[0]);
  const def = $derived(unit && COSTUMES.get(unit.id));
  const ch = $derived(unit && characterOf(unit.id));
  const bond = $derived(unit ? (session.profile.bonds[ch.id] ?? { exp: 0, level: 0 }) : null);
  const cap = $derived(unit ? levelCap(unit.ascension) : 0);
  const stats = $derived(unit ? unitStats(unit.id, unit.level, unit.ascension, bond.level) : null);

  /** 錢夠的話最多能升幾級（不超過上限） */
  function affordable(n) {
    const to = Math.min(cap, unit.level + n);
    const c = levelCost(unit.level, to);
    return { to, ...c, ok: to > unit.level && c.gold <= session.profile.gold && c.notes <= session.profile.notes };
  }
  const steps = $derived(unit ? [1, 5, 10, cap - unit.level].filter((n, i, a) => n > 0 && a.indexOf(n) === i).map((n) => ({ n, ...affordable(n) })) : []);

  const canAscend = $derived(unit && unit.ascension < 5 && unit.dupes > 0 && unit.level >= cap && session.profile.gold >= ascendCost(unit.ascension));

  const run = (fn) => fn().catch(() => {});
</script>

<TopBar title={t('角色')} en="Roster" num="03" {onback} />

{#if unit}
  <section class="detail el-{def.element}">
    <Panel tone="ink" from="left" pad={false}>
      <div class="head">
        <img class="por" src={portrait(ch.id, 'happy')} alt="" draggable="false" />
        <div class="who">
          <span class="stars rarity-{def.rarity}">{'★'.repeat(def.rarity)}</span>
          <h3>{t(ch.name)}<small>{t(def.name)}</small></h3>
          <p class="tags">
            <span class="el-mark">{t(ELEMENT_NAME[def.element])}</span>
            <Tag variant="blue" size="sm">{t(ROLE_NAME[def.role])}</Tag>
          </p>
          <p class="lv">Lv.<b>{unit.level}</b><span>／{cap}</span></p>
          <p class="sub">{t('突破')} {unit.ascension}／5（{t('存 {n}', { n: unit.dupes })}）・{t('羈絆')} {bond.level}（{bond.exp}）</p>
        </div>
      </div>
      <div class="body">
        <div class="actions">
          {#each steps as s (s.n)}
            <Button shape="rect" size="sm" disabled={!s.ok || session.busy} onclick={() => run(() => levelUp(unit.id, s.to - unit.level))}>
              Lv {s.to}<small>● {s.gold}・▤ {s.notes}</small>
            </Button>
          {/each}
          <Button shape="rect" size="sm" variant="secondary" disabled={!canAscend || session.busy} onclick={() => run(() => ascend(unit.id))}>
            {t('突破')}<small>● {ascendCost(unit.ascension)}</small>
          </Button>
        </div>
        <StatList
          columns={2}
          items={[
            { icon: '♥', label: 'HP', value: stats.hp },
            { icon: '»', label: t('速度'), value: stats.spd },
            { icon: '⚔', label: t('攻擊'), value: stats.atk },
            { icon: '◆', label: t('防禦'), value: stats.def },
            { icon: '✦', label: t('暴擊'), value: `${stats.crit}%` },
            { icon: '✧', label: t('暴擊傷害'), value: `${stats.crit_dmg}%` },
            { icon: '◎', label: t('效果命中'), value: `${stats.hit}%` },
            { icon: '◇', label: t('效果抵抗'), value: `${stats.resist}%` },
          ]}
        />
        <ul class="skills">
          {#each def.skills as id, i (id)}
            {@const s = SKILLS.get(id)}
            <li>
              <b><span class="slot">S{i + 1}</span>{t(s.name)}<small>{cooldownText(s)}</small></b>
              <span>{skillText(s)}</span>
            </li>
          {/each}
          {#each def.passives ?? [] as p (p.name)}
            <li>
              <b><span class="slot">P</span>{t(p.name)}</b>
              <span>{passiveText(p)}</span>
            </li>
          {/each}
        </ul>
      </div>
    </Panel>
  </section>
{/if}

<section class="list">
  <SectionBar title={t('手上的角色')} en={`${owned.length} Units`} />
  <div class="grid">
    {#each owned as c, i (c.id)}
      <UnitCard costume={c.id} level={c.level} num={i + 1} selected={unit?.id === c.id} onclick={() => (picked = c.id)} />
    {/each}
  </div>
  <p class="hint">{t('突破要先練到等級上限，並且再抽到同一個角色。同一位角色的所有版本共用羈絆：練任何一個都會加，羈絆每一級全能力 +1%。')}</p>
</section>

<style>
  .detail {
    position: absolute;
    left: 40px;
    top: 112px;
    bottom: 28px;
    width: 520px;
  }

  .detail :global(.bb-panel) {
    height: 100%;
    overflow: hidden;
  }

  .head {
    position: relative;
    display: flex;
    height: 180px;
    overflow: hidden;
    background: linear-gradient(110deg, color-mix(in srgb, var(--el) 70%, #000) 0 40%, transparent 40%), linear-gradient(180deg, #1b2440, #0b0f1a);
  }

  .por {
    position: absolute;
    left: -10px;
    top: -6px;
    height: 260px;
  }

  .who {
    margin-left: 200px;
    padding: 14px 16px 0 0;
    color: var(--bb-on-ink);
  }

  .stars {
    color: var(--rarity);
    letter-spacing: 2px;
  }

  h3 {
    margin: 2px 0 6px;
    font-size: 30px;
    font-weight: 900;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  h3 small {
    font-size: 15px;
    color: var(--bb-yellow);
  }

  .tags {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
  }

  .lv {
    margin: 8px 0 0;
    font-size: 14px;
  }

  .lv b {
    font: italic 400 34px/1 var(--bb-font-display);
    margin-left: 4px;
  }

  .lv span {
    color: var(--bb-on-ink-2);
  }

  .sub {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--bb-on-ink-2);
  }

  .body {
    padding: 14px 16px 16px;
    color: var(--bb-on-ink);
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: calc(100% - 180px);
    overflow-y: auto;
  }

  .skills {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }

  .skills li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px;
    background: rgb(255 255 255 / 0.06);
    border-left: 3px solid var(--bb-blue-2);
    font-size: 13px;
  }

  .skills b {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 14px;
  }

  .slot {
    font: italic 400 15px var(--bb-font-display);
    color: var(--bb-yellow);
  }

  .skills small {
    margin-left: auto;
    font-weight: 400;
    color: var(--bb-on-ink-2);
    font-size: 11px;
  }

  .skills li > span {
    color: var(--bb-on-ink-2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .actions small {
    margin-left: 8px;
    font-size: 11px;
    font-weight: 400;
    opacity: 0.85;
  }

  .list {
    position: absolute;
    left: 590px;
    right: 40px;
    top: 112px;
    bottom: 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(5, 120px);
    gap: 14px 12px;
    align-content: start;
    overflow-y: auto;
    padding: 4px;
    flex: 1;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--bb-ink-2);
  }
</style>
