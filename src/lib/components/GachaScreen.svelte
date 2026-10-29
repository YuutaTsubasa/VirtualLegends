<script>
  // 招募：結果由 Server 決定，這裡只演出。機率（每個角色）與保底都公開。
  import { session, gacha } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, unitName } from '../data.js';
  import { portrait } from '../characters.js';
  import TopBar from './TopBar.svelte';
  import UnitCard from './UnitCard.svelte';

  let { onback } = $props();

  const banner = $derived(session.banners.find((b) => b.id === 'standard'));
  let results = $state(null);
  let revealed = $state(0);
  let showRates = $state(false);

  const featured = $derived(banner?.cards.filter((c) => c.rarity === 5) ?? []);
  const untilPity = $derived(banner && session.profile ? banner.pity - session.profile.pity : 0);

  const LABEL = { new: 'NEW', dupe: '突破 +1', overflow: '星屑' };

  async function pull(count) {
    try {
      results = await gacha(banner.id, count);
      revealed = 0;
      for (let i = 1; i <= results.length; i++) {
        await new Promise((r) => setTimeout(r, results[i - 1].rarity >= 4 ? 420 : 180));
        revealed = i;
      }
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }

  // 萬分之一 → 百分比（300 → 3%、82 → 0.82%）
  const pct = (rate) => `${+(rate / 100).toFixed(3)}%`;
</script>

<TopBar title={t('招募')} {onback} />

{#if banner}
  <section class="banner">
    <div class="art" aria-hidden="true">
      {#each featured as c, i (c.costume)}
        <img src={portrait(COSTUMES.get(c.costume).character, 'win')} alt="" style:--i={i} draggable="false" />
      {/each}
    </div>
    <div class="info panel">
      <h3>{t(banner.name)}</h3>
      <p>{t('★5 角色：')}{featured.map((c) => unitName(c.costume)).join('、')}</p>
      <p class="rates">
        ★5 {pct(banner.rates[5])}・★4 {pct(banner.rates[4])}・★3 {pct(banner.rates[3])}
        <button type="button" class="link" onclick={() => (showRates = true)}>{t('每個角色的機率')}</button>
      </p>
      <p>{t('再 {n} 抽必定出現 ★5；十連至少一個 ★4 以上。', { n: untilPity })}</p>
      <div class="buttons">
        <button class="btn sub" type="button" disabled={session.busy || session.profile.stardust < banner.cost} onclick={() => pull(1)}>
          {t('招募 1 次')}<small>✦ {banner.cost}</small>
        </button>
        <button class="btn" type="button" disabled={session.busy || session.profile.stardust < banner.ten_cost} onclick={() => pull(10)}>
          {t('招募 10 次')}<small>✦ {banner.ten_cost}</small>
        </button>
      </div>
    </div>
  </section>
{/if}

{#if results}
  <div class="scrim" role="dialog" aria-modal="true" aria-label={t('招募結果')}>
    <div class="grid" class:single={results.length === 1}>
      {#each results as r, i (i)}
        <div class="slot" class:shown={i < revealed} class:shine={r.rarity >= 4}>
          {#if i < revealed}
            <UnitCard costume={r.costume} badge={t(LABEL[r.result])} />
          {:else}
            <div class="back rarity-{r.rarity}"></div>
          {/if}
        </div>
      {/each}
    </div>
    <button class="btn" type="button" disabled={revealed < results.length} onclick={() => (results = null)}>{t('確定')}</button>
  </div>
{/if}

{#if showRates && banner}
  <div class="scrim" role="dialog" aria-modal="true" aria-label={t('機率一覽')}>
    <div class="panel rates-list">
      <h3>{t('機率一覽')}・{t(banner.name)}</h3>
      <table>
        <tbody>
          {#each [...banner.cards].sort((a, b) => b.rarity - a.rarity) as c (c.costume)}
            <tr><td>{'★'.repeat(c.rarity)}</td><td>{unitName(c.costume)}</td><td class="num">{pct(c.rate)}</td></tr>
          {/each}
        </tbody>
      </table>
      <p>{t('保底：距離上一個 ★5 第 {n} 次必定是 ★5。十連的最後一個若前九個都沒有 ★4 以上，會換成 ★4。重複抽到的角色會變成突破次數，滿突破後換成星屑與羈絆經驗。', { n: banner.pity })}</p>
      <button class="btn" type="button" onclick={() => (showRates = false)}>{t('關閉')}</button>
    </div>
  </div>
{/if}

<style>
  .banner {
    position: absolute;
    inset: 10cqh 4cqh 4cqh;
    display: flex;
    align-items: stretch;
    gap: 3cqh;
  }

  .art {
    flex: 1;
    position: relative;
    border: var(--line) solid var(--gold);
    border-radius: 1.6cqh;
    overflow: hidden;
    background: radial-gradient(circle at 40% 40%, #7d5cf0 0%, #2a1a6e 60%, #0e0830 100%);
  }

  .art img {
    position: absolute;
    bottom: -4cqh;
    height: 90cqh;
    left: calc(var(--i) * 38cqh - 8cqh);
    filter: drop-shadow(0 0 2cqh rgba(255, 207, 74, 0.6));
  }

  .info {
    width: 62cqh;
    padding: 2.4cqh 3cqh;
    display: flex;
    flex-direction: column;
  }

  .info h3 {
    margin: 0 0 1cqh;
    font-size: 1.4em;
  }

  .info p {
    margin: 0.8cqh 0;
    font-size: 0.85em;
  }

  .rates {
    font-weight: 700;
  }

  .link {
    background: none;
    border: none;
    color: var(--pin);
    text-decoration: underline;
    padding: 0 0 0 1cqh;
  }

  .buttons {
    margin-top: auto;
    display: flex;
    gap: 2cqh;
  }

  .buttons .btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .buttons small {
    font-size: 0.75em;
    color: #c7ecff;
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3cqh;
    z-index: 20;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(5, 15cqh);
    gap: 2cqh;
  }

  .grid.single {
    grid-template-columns: 15cqh;
  }

  .slot {
    width: 15cqh;
    height: 21cqh;
  }

  .slot.shown.shine {
    animation: shine 0.6s ease-out;
  }

  @keyframes shine {
    0% {
      transform: scale(1.25);
      filter: brightness(2.4);
    }
    100% {
      transform: scale(1);
      filter: brightness(1);
    }
  }

  .back {
    width: 100%;
    height: 100%;
    border-radius: 1cqh;
    border: 0.4cqh solid var(--gold-deep);
    background: repeating-linear-gradient(45deg, var(--felt) 0 1.2cqh, var(--felt-soft) 1.2cqh 2.4cqh);
  }

  .rates-list {
    width: 80cqh;
    max-height: 84cqh;
    overflow: auto;
    padding: 2.4cqh 3cqh;
    font-size: 0.85em;
  }

  .rates-list h3 {
    margin-top: 0;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  td {
    padding: 0.4cqh 1cqh;
    border-bottom: 0.15cqh solid var(--ivory-deep);
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
