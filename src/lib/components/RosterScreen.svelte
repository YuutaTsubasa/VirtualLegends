<script>
  // 角色一覽與養成：升級、突破。花費與上限 Server 會再檢查一次，這裡只是先算給玩家看。
  import { session, levelUp, ascend } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, SKILLS, SUIT_NAME, ROLE_NAME, unitName, characterOf, unitStats, levelCap, levelCost, ascendCost, skillText, skillCost } from '../data.js';
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

<TopBar title={t('角色')} {onback} />

<section class="list">
  {#each owned as c (c.id)}
    <UnitCard costume={c.id} level={c.level} selected={unit?.id === c.id} onclick={() => (picked = c.id)} />
  {/each}
</section>

{#if unit}
  <section class="detail panel">
    <img class="por" src={portrait(ch.id, 'happy')} alt="" draggable="false" />
    <div class="body">
      <h3>
        <span class="stars rarity-{def.rarity}">{'★'.repeat(def.rarity)}</span>
        {unitName(unit.id)}
      </h3>
      <p class="tags">
        <span class="tag suit-{def.suit}">{t(SUIT_NAME[def.suit])}</span>
        <span class="tag">{t(ROLE_NAME[def.role])}</span>
        <span>{t(ch.intro)}</span>
      </p>
      <dl class="stats">
        <div><dt>Lv</dt><dd>{unit.level}／{cap}</dd></div>
        <div><dt>{t('突破')}</dt><dd>{unit.ascension}／5（{t('存 {n}', { n: unit.dupes })}）</dd></div>
        <div><dt>{t('羈絆')}</dt><dd>{bond.level}（{bond.exp}）</dd></div>
        <div><dt>HP</dt><dd>{stats.hp}</dd></div>
        <div><dt>{t('攻擊')}</dt><dd>{stats.atk}</dd></div>
        <div><dt>{t('防禦')}</dt><dd>{stats.def}</dd></div>
        <div><dt>{t('速度')}</dt><dd>{stats.spd}</dd></div>
      </dl>
      <ul class="skills">
        {#each def.skills as id, i (id)}
          {@const s = SKILLS.get(id)}
          <li>
            <b>{[t('普攻'), t('技能'), t('大招')][i]}・{t(s.name)}</b>
            <span>{skillCost(s)}</span>
            <span>{skillText(s)}</span>
          </li>
        {/each}
      </ul>
      <div class="actions">
        {#each steps as s (s.n)}
          <button class="btn sub" type="button" disabled={!s.ok || session.busy} onclick={() => run(() => levelUp(unit.id, s.to - unit.level))}>
            Lv {s.to}<small>● {s.gold}・▤ {s.notes}</small>
          </button>
        {/each}
        <button class="btn" type="button" disabled={!canAscend || session.busy} onclick={() => run(() => ascend(unit.id))}>
          {t('突破')}<small>● {ascendCost(unit.ascension)}</small>
        </button>
      </div>
      <p class="hint">
        {t('突破要先練到等級上限，並且再抽到同一個角色。同一位角色的所有版本共用羈絆：練任何一個都會加，羈絆每一級全能力 +1%。')}
      </p>
    </div>
  </section>
{/if}

<style>
  .list {
    position: absolute;
    top: 10cqh;
    bottom: 3cqh;
    left: 3cqh;
    width: 70cqh;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(4, 15cqh);
    gap: 2cqh;
    align-content: start;
    padding: 1cqh;
  }

  .detail {
    position: absolute;
    top: 10cqh;
    bottom: 3cqh;
    left: 76cqh;
    right: 3cqh;
    display: flex;
    overflow: hidden;
  }

  .por {
    width: 36cqh;
    object-fit: cover;
    object-position: 50% 8%;
    background: radial-gradient(circle at 50% 30%, #fff 0%, var(--ivory-deep) 80%);
  }

  .body {
    flex: 1;
    padding: 2cqh 2.6cqh;
    overflow-y: auto;
    font-size: 0.85em;
  }

  h3 {
    margin: 0;
    font-size: 1.4em;
  }

  .stars {
    color: color-mix(in srgb, var(--rarity) 70%, #6b4a00);
  }

  .tags {
    display: flex;
    gap: 1cqh;
    align-items: center;
    margin: 0.8cqh 0;
  }

  .tag {
    background: var(--suit, var(--felt));
    color: var(--ivory);
    padding: 0.2cqh 1cqh;
    border-radius: 0.6cqh;
    font-weight: 700;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.6cqh 2cqh;
    white-space: nowrap;
    margin: 1cqh 0;
  }

  .stats div {
    display: flex;
    justify-content: space-between;
    border-bottom: 0.15cqh solid var(--ivory-deep);
  }

  dt {
    color: #6c5f45;
  }

  dd {
    margin: 0;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .skills {
    list-style: none;
    padding: 0;
    margin: 1cqh 0;
    display: grid;
    gap: 0.8cqh;
  }

  .skills li {
    display: flex;
    flex-direction: column;
    background: rgba(15, 59, 46, 0.08);
    border-radius: 0.8cqh;
    padding: 0.6cqh 1.2cqh;
  }

  .skills span {
    font-size: 0.9em;
    color: #4a4135;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 1cqh;
  }

  .actions .btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0.8cqh 2cqh;
    font-size: 0.95em;
  }

  .actions small {
    font-size: 0.75em;
    font-weight: 400;
  }

  .hint {
    color: #6c5f45;
    font-size: 0.85em;
  }
</style>
