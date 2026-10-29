<script>
  // 大廳：左邊巨大階梯選單（選了看右邊說明，按「進入」或 Enter 才走），右邊站手上稀有度最高的角色
  import { StairMenu, Panel, Button, Modal, Ticker } from 'bluebolt-ui';
  import { session, issueTransfer } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { portrait } from '../characters.js';
  import { COSTUMES, unitName } from '../data.js';
  import TopBar from './TopBar.svelte';

  let { go } = $props();
  let transfer = $state(null);
  let transferOpen = $state(false);
  let active = $state('stages');

  const items = $derived([
    { id: 'stages', en: 'Sortie', zh: t('出擊') },
    { id: 'gacha', en: 'Recruit', zh: t('招募') },
    { id: 'roster', en: 'Roster', zh: t('角色') },
    { id: 'transfer', en: 'Transfer', zh: t('繼承碼') },
  ]);

  const INFO = {
    stages: { title: '出擊', text: '主線第一章「新傳說的入口」。排好出手順序與技能時機，打倒裂界守門者。' },
    gacha: { title: '招募', text: '用星屑招募新角色。每個角色的機率與保底都公開。' },
    roster: { title: '角色', text: '升級、突破，查看技能、被動與能力值。' },
    transfer: { title: '繼承碼', text: '換裝置時用：在新裝置的標題畫面輸入，就能取回這個帳號。' },
  };

  // 主畫面站的是手上稀有度最高的角色
  const leader = $derived.by(() => {
    const list = session.profile?.costumes ?? [];
    return [...list].sort((a, b) => COSTUMES.get(b.id).rarity - COSTUMES.get(a.id).rarity)[0]?.id ?? 'xiaocui.daily';
  });

  async function enter(id = active) {
    if (id !== 'transfer') return go(id);
    try {
      transfer = await issueTransfer();
      transferOpen = true;
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }

  function onkeydown(e) {
    if (e.key === 'Enter' && !transferOpen && document.activeElement?.closest('.bb-stair')) enter();
  }
</script>

<svelte:window {onkeydown} />

<TopBar title={t('大廳')} en="Lobby" num="00" />

<img class="leader" src={portrait(COSTUMES.get(leader).character, 'happy')} alt="" draggable="false" />
<p class="leader-name">{unitName(leader)}</p>

<div class="menu">
  <StairMenu {items} bind:active label={t('主選單')} />
</div>

<div class="info">
  <Panel num={String(items.findIndex((i) => i.id === active) + 1).padStart(2, '0')} title={t(INFO[active].title)} sub={items.find((i) => i.id === active)?.en} tone="glass" from="left">
    <p>{t(INFO[active].text)}</p>
    {#if active === 'roster'}<p class="count">{t('手上 {n} 名角色', { n: session.profile?.costumes.length ?? 0 })}</p>{/if}
    <Button en="Enter" onclick={() => enter()}>{t('進入')}</Button>
  </Panel>
</div>

{#if session.daily}
  <div class="daily">
    <Panel num="✦" title={t('每日登入獎勵')} sub="Daily" tone="ink" from="right">
      <p>{t('星屑 {s}、金幣 {g}', { s: session.daily.stardust, g: session.daily.gold })}</p>
      <Button size="sm" variant="accent" onclick={() => (session.daily = null)}>{t('收下')}</Button>
    </Panel>
  </div>
{/if}

<div class="ticker"><Ticker fixed={false} build="VL-M1" log={[t('第一章：新傳說的入口'), t('招募機率公開'), t('不收真錢')]} keys={[{ k: '↑↓', v: t('選擇') }, { k: 'Enter', v: t('進入') }]} /></div>

<Modal bind:open={transferOpen} title={t('繼承碼')} sub="Transfer Code" width="560px" actions={[{ label: t('關閉'), close: true }]}>
  {#if transfer}
    <p class="code">{transfer.code}</p>
    <p>{t('在新裝置的標題畫面輸入，就能取回這個帳號（7 天內有效，用過一次就失效；這台裝置之後要用新的繼承碼才能再登入）。')}</p>
  {/if}
</Modal>

<style>
  .leader {
    position: absolute;
    right: -30px;
    bottom: 0;
    height: 660px;
    filter: drop-shadow(0 12px 24px rgb(11 15 26 / 0.35));
    animation: rise var(--bb-dur-5) var(--bb-ease) both;
  }

  @keyframes rise {
    from {
      transform: translateX(calc(80px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .leader-name {
    position: absolute;
    right: 40px;
    bottom: 56px;
    margin: 0;
    padding: 4px 16px;
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    font-weight: 800;
    transform: skewX(var(--bb-skew));
  }

  .menu {
    position: absolute;
    left: 40px;
    top: 120px;
  }

  .info {
    position: absolute;
    left: 40px;
    top: 470px;
    width: 470px;
  }

  .info p {
    margin: 0 0 12px;
    color: var(--bb-ink-2);
  }

  .info .count {
    font-weight: 800;
    color: var(--bb-ink);
  }

  .daily {
    position: absolute;
    right: 340px;
    top: 120px;
    width: 300px;
  }

  .daily p {
    margin: 0 0 10px;
  }

  .ticker {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
  }

  .code {
    font: 700 40px var(--bb-font-mono);
    letter-spacing: 0.2em;
    text-align: center;
    margin: 8px 0 16px;
    color: var(--bb-blue-text);
  }
</style>
