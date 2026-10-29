<script>
  // 招募：結果由 Server 決定，這裡只演出。機率（每個角色）與保底都公開。
  import { Panel, Button, Modal, CutIn, Tag, Gauge } from 'bluebolt-ui';
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
  let cutin = $state(false);

  const featured = $derived(banner?.cards.filter((c) => c.rarity === 5) ?? []);
  const untilPity = $derived(banner && session.profile ? banner.pity - session.profile.pity : 0);

  const LABEL = { new: 'NEW', dupe: '突破 +1', overflow: '星屑' };

  async function pull(count) {
    try {
      results = await gacha(banner.id, count);
      revealed = 0;
      if (results.some((r) => r.rarity === 5)) {
        cutin = true;
        await new Promise((r) => setTimeout(r, 1300));
      }
      for (let i = 1; i <= results.length; i++) {
        await new Promise((r) => setTimeout(r, results[i - 1].rarity >= 4 ? 380 : 160));
        revealed = i;
      }
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }

  // 萬分之一 → 百分比（300 → 3%、82 → 0.82%）
  const pct = (rate) => `${+(rate / 100).toFixed(3)}%`;
</script>

<TopBar title={t('招募')} en="Recruit" num="02" {onback} />

{#if banner}
  <div class="art" aria-hidden="true">
    {#each featured as c, i (c.costume)}
      <img src={portrait(COSTUMES.get(c.costume).character, 'win')} alt="" style:--i={i} draggable="false" />
    {/each}
    <span class="pickup">PICK UP ★5</span>
  </div>

  <div class="info">
    <Panel num="01" title={t(banner.name)} sub="Standard Banner" tone="glass" from="left">
      <p class="feat">
        <Tag variant="yellow">★5</Tag>
        {featured.map((c) => unitName(c.costume)).join('、')}
      </p>
      <dl class="rates">
        <div><dt>★5</dt><dd>{pct(banner.rates[5])}</dd></div>
        <div><dt>★4</dt><dd>{pct(banner.rates[4])}</dd></div>
        <div><dt>★3</dt><dd>{pct(banner.rates[3])}</dd></div>
      </dl>
      <div class="pity">
        <Gauge value={(banner.pity - untilPity) / banner.pity} label={t('保底')} en="Pity" segments={16} showValue={false} />
        <p>{t('再 {n} 抽必定出現 ★5；十連至少一個 ★4 以上。', { n: untilPity })}</p>
      </div>
      <Button shape="rect" variant="ghost" size="sm" onclick={() => (showRates = true)}>{t('每個角色的機率')}</Button>
    </Panel>

    <div class="buttons">
      <Button variant="ink" size="lg" en={`✦ ${banner.cost}`} disabled={session.busy || session.profile.stardust < banner.cost} onclick={() => pull(1)}>
        {t('招募 1 次')}
      </Button>
      <Button size="lg" en={`✦ ${banner.ten_cost}`} disabled={session.busy || session.profile.stardust < banner.ten_cost} onclick={() => pull(10)}>
        {t('招募 10 次')}
      </Button>
    </div>
  </div>
{/if}

<CutIn bind:show={cutin} title={t('傳說降臨')} en="LEGEND ★5" num="★" tone="yellow" duration={1200} />

{#if results}
  <div class="results" role="dialog" aria-modal="true" aria-label={t('招募結果')}>
    <div class="grid" class:single={results.length === 1}>
      {#each results as r, i (i)}
        <div class="slot" class:shown={i < revealed} class:shine={r.rarity >= 4} class:legend={r.rarity === 5}>
          {#if i < revealed}
            <UnitCard costume={r.costume} badge={t(LABEL[r.result])} width={176} />
          {:else}
            <div class="back rarity-{r.rarity}"><span>VL</span></div>
          {/if}
        </div>
      {/each}
    </div>
    <Button size="lg" disabled={revealed < results.length} onclick={() => (results = null)}>{t('確定')}</Button>
  </div>
{/if}

<Modal bind:open={showRates} title={t('機率一覽')} sub={banner ? t(banner.name) : ''} width="640px" actions={[{ label: t('關閉'), close: true }]}>
  {#if banner}
    <table>
      <tbody>
        {#each [...banner.cards].sort((a, b) => b.rarity - a.rarity) as c (c.costume)}
          <tr><td class="r{c.rarity}">{'★'.repeat(c.rarity)}</td><td>{unitName(c.costume)}</td><td class="num">{pct(c.rate)}</td></tr>
        {/each}
      </tbody>
    </table>
    <p class="fine">{t('保底：距離上一個 ★5 第 {n} 次必定是 ★5。十連的最後一個若前九個都沒有 ★4 以上，會換成 ★4。重複抽到的角色會變成突破次數，滿突破後換成星屑與羈絆經驗。', { n: banner.pity })}</p>
  {/if}
</Modal>

<style>
  .art {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 760px;
    overflow: hidden;
  }

  .art img {
    position: absolute;
    bottom: -20px;
    height: 640px;
    right: calc(var(--i) * 300px - 60px);
    filter: drop-shadow(0 12px 24px rgb(11 15 26 / 0.4));
  }

  .pickup {
    position: absolute;
    right: 40px;
    top: 120px;
    font: italic 400 64px/1 var(--bb-font-display);
    color: transparent;
    -webkit-text-stroke: 2px var(--bb-blue);
  }

  .info {
    position: absolute;
    left: 40px;
    top: 120px;
    width: 500px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .feat {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 12px;
    font-weight: 800;
  }

  .rates {
    display: flex;
    gap: 10px;
    margin: 0 0 14px;
  }

  .rates div {
    flex: 1;
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    padding: 6px 12px;
    clip-path: polygon(8px 0, 100% 0, calc(100% - 8px) 100%, 0 100%);
  }

  .rates dt {
    font-size: 12px;
    color: var(--bb-yellow);
  }

  .rates dd {
    margin: 0;
    font: italic 400 26px var(--bb-font-display);
  }

  .pity p {
    margin: 6px 0 12px;
    font-size: 14px;
    color: var(--bb-ink-2);
  }

  .buttons {
    display: flex;
    gap: 16px;
  }

  .results {
    position: absolute;
    inset: 0;
    z-index: 20;
    background: rgb(6 11 26 / 0.82);
    backdrop-filter: blur(6px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(5, 176px);
    gap: 16px;
  }

  .grid.single {
    grid-template-columns: 176px;
  }

  .slot {
    height: 243px;
    perspective: 600px;
  }

  .slot.shown {
    animation: flip 0.35s var(--bb-ease);
  }

  .slot.shine.shown {
    filter: drop-shadow(0 0 14px var(--r4));
  }

  .slot.legend.shown {
    filter: drop-shadow(0 0 18px var(--r5));
  }

  @keyframes flip {
    from {
      transform: rotateY(90deg);
    }
  }

  .back {
    height: 100%;
    display: grid;
    place-items: center;
    background:
      repeating-linear-gradient(135deg, rgb(255 255 255 / 0.06) 0 8px, transparent 8px 16px),
      linear-gradient(160deg, var(--bb-blue), #0a1a6b);
    border-bottom: 4px solid var(--rarity);
    clip-path: polygon(0 0, 100% 0, 100% 100%, 12px 100%, 0 calc(100% - 12px));
  }

  .back span {
    font: italic 400 56px var(--bb-font-display);
    color: rgb(255 255 255 / 0.25);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }

  td {
    padding: 5px 8px;
    border-bottom: 1px solid var(--bb-line);
  }

  .r5 {
    color: #c79a00;
  }
  .r4 {
    color: #8a5cff;
  }

  .num {
    text-align: right;
    font-family: var(--bb-font-mono);
  }

  .fine {
    font-size: 12px;
    color: var(--bb-ink-2);
  }
</style>
