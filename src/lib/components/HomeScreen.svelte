<script>
  import { session, issueTransfer } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { portrait } from '../characters.js';
  import { COSTUMES } from '../data.js';
  import TopBar from './TopBar.svelte';

  let { go } = $props();
  let transfer = $state(null);

  // 主畫面站的是手上稀有度最高的角色
  const leader = $derived.by(() => {
    const list = session.profile?.costumes ?? [];
    const best = [...list].sort((a, b) => COSTUMES.get(b.id).rarity - COSTUMES.get(a.id).rarity)[0];
    return best ? COSTUMES.get(best.id).character : 'xiaocui';
  });

  async function showTransfer() {
    try {
      transfer = await issueTransfer();
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }
</script>

<TopBar title={t('大廳')} />

<img class="leader" src={portrait(leader, 'happy')} alt="" draggable="false" />

{#if session.daily}
  <div class="daily panel" role="status">
    <b>{t('每日登入獎勵')}</b>
    <span>{t('星屑 {s}、金幣 {g}', { s: session.daily.stardust, g: session.daily.gold })}</span>
    <button class="btn sub" type="button" onclick={() => (session.daily = null)}>{t('收下')}</button>
  </div>
{/if}

<nav>
  <button class="big" type="button" onclick={() => go('stages')}>
    <b>{t('出擊')}</b><small>{t('主線第一章')}</small>
  </button>
  <button class="big gacha" type="button" onclick={() => go('gacha')}>
    <b>{t('招募')}</b><small>{t('常駐招募')}</small>
  </button>
  <button class="big sub" type="button" onclick={() => go('roster')}>
    <b>{t('角色')}</b><small>{t('{n} 名', { n: session.profile?.costumes.length ?? 0 })}</small>
  </button>
  <button class="small" type="button" onclick={showTransfer}>{t('繼承碼')}</button>
</nav>

{#if transfer}
  <div class="scrim" role="dialog" aria-modal="true">
    <div class="panel dialog">
      <h3>{t('繼承碼')}</h3>
      <p class="code">{transfer.code}</p>
      <p>{t('在新裝置的標題畫面輸入，就能取回這個帳號（7 天內有效，用過一次就失效；這台裝置之後要用新的繼承碼才能再登入）。')}</p>
      <button class="btn" type="button" onclick={() => (transfer = null)}>{t('關閉')}</button>
    </div>
  </div>
{/if}

<style>
  .leader {
    position: absolute;
    left: 4cqh;
    bottom: 0;
    height: 88cqh;
    filter: drop-shadow(0 0 3cqh rgba(0, 0, 0, 0.7));
  }

  nav {
    position: absolute;
    right: 5cqh;
    top: 14cqh;
    width: 70cqh;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2.4cqh;
  }

  .big {
    height: 26cqh;
    border: var(--line) solid var(--gold);
    border-radius: 1.6cqh;
    background: linear-gradient(135deg, var(--accent) 0%, var(--accent-deep) 100%);
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    padding: 2.4cqh 3cqh;
    box-shadow: 0 0.8cqh 0 rgba(0, 0, 0, 0.4);
    text-align: left;
  }

  .big:first-child {
    grid-column: span 2;
    background: linear-gradient(135deg, #e0583f 0%, var(--accent-deep) 100%);
  }

  .big.gacha {
    background: linear-gradient(135deg, #6a4ed8 0%, #2f1f7a 100%);
  }

  .big.sub {
    background: linear-gradient(135deg, var(--felt-soft) 0%, var(--felt-deep) 100%);
  }

  .big b {
    font-size: 2em;
    letter-spacing: 0.2em;
  }

  .big small {
    color: var(--gold);
  }

  .big:hover {
    filter: brightness(1.12);
  }

  .small {
    grid-column: span 2;
    justify-self: end;
    background: rgba(0, 0, 0, 0.35);
    border: 0.2cqh solid var(--gold);
    border-radius: 0.8cqh;
    padding: 0.8cqh 2cqh;
  }

  .daily {
    position: absolute;
    top: 11cqh;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 2cqh;
    padding: 1.2cqh 2.4cqh;
    z-index: 6;
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    display: grid;
    place-items: center;
    z-index: 20;
  }

  .dialog {
    width: 70cqh;
    padding: 3cqh 4cqh;
    text-align: center;
  }

  .dialog h3 {
    margin: 0;
  }

  .code {
    font-size: 2em;
    letter-spacing: 0.25em;
    font-weight: 800;
    color: var(--accent-deep);
    -webkit-user-select: all;
    user-select: all;
  }

  .dialog p {
    font-size: 0.85em;
  }
</style>
