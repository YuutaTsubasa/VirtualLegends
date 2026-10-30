<script>
  // 大廳：背景是基地的休息室，中間站手上稀有度最高的角色（點一下會換表情），右邊是大按鈕選單，點了直接過去。
  import { Panel, Button, Modal, Ticker } from 'bluebolt-ui';
  import { session, issueTransfer } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { COSTUMES, unitName } from '../data.js';
  import { costumeArt, spriteArt, sceneArt } from '../art.js';
  import TopBar from './TopBar.svelte';

  let { go } = $props();
  let transfer = $state(null);
  let transferOpen = $state(false);
  let mood = $state('normal');

  const MENU = [
    { id: 'stages', en: 'Sortie', zh: '出擊', sub: '主線第一章', tone: 'blue', big: true },
    { id: 'gacha', en: 'Recruit', zh: '招募', sub: '常駐招募', tone: 'yellow' },
    { id: 'roster', en: 'Roster', zh: '角色', sub: '養成・技能', tone: 'ink' },
  ];

  // 主畫面站的是手上稀有度最高的角色
  const leader = $derived.by(() => {
    const list = session.profile?.costumes ?? [];
    return [...list].sort((a, b) => COSTUMES.get(b.id).rarity - COSTUMES.get(a.id).rarity)[0]?.id ?? 'xiaocui.daily';
  });
  const full = $derived(spriteArt(leader));
  const bg = sceneArt('lobby_bg');

  function poke() {
    mood = mood === 'happy' ? 'win' : 'happy';
    setTimeout(() => (mood = 'normal'), 1600);
  }

  async function showTransfer() {
    try {
      transfer = await issueTransfer();
      transferOpen = true;
    } catch {
      /* 錯誤訊息在 session.error */
    }
  }
</script>

{#if bg}<img class="bg" src={bg} alt="" draggable="false" />{/if}
<span class="floor" aria-hidden="true"></span>
<span class="topshade" aria-hidden="true"></span>

<TopBar title={t('大廳')} en="Lobby" num="00" />

<button class="leader" class:full type="button" onclick={poke} aria-label={unitName(leader)}>
  <img src={full ?? costumeArt(leader, mood)} alt="" draggable="false" />
</button>
<p class="leader-name"><b>{unitName(leader)}</b><small>{t('點一下打招呼')}</small></p>

<nav class="menu" aria-label={t('主選單')}>
  {#each MENU as m, i (m.id)}
    <button type="button" class="tile t-{m.tone}" class:big={m.big} style:--i={i} onclick={() => go(m.id)}>
      <span class="en">{m.en}</span>
      <b>{t(m.zh)}</b>
      <small>{m.id === 'roster' ? t('手上 {n} 名', { n: session.profile?.costumes.length ?? 0 }) : t(m.sub)}</small>
      <span class="arrow" aria-hidden="true">▶</span>
    </button>
  {/each}
  <div class="small">
    <Button shape="rect" variant="ghost" size="sm" onclick={showTransfer}>{t('繼承碼')}</Button>
  </div>
</nav>

{#if session.daily}
  <div class="daily">
    <Panel num="✦" title={t('每日登入獎勵')} sub="Daily" tone="ink" from="left">
      <p>{t('星屑 {s}、金幣 {g}', { s: session.daily.stardust, g: session.daily.gold })}</p>
      <Button size="sm" variant="accent" onclick={() => (session.daily = null)}>{t('收下')}</Button>
    </Panel>
  </div>
{/if}

<div class="ticker"><Ticker fixed={false} build="VL-M1" log={[t('第一章：新傳說的入口'), t('招募機率公開'), t('不收真錢')]} /></div>

<Modal bind:open={transferOpen} title={t('繼承碼')} sub="Transfer Code" width="560px" actions={[{ label: t('關閉'), close: true }]}>
  {#if transfer}
    <p class="code">{transfer.code}</p>
    <p>{t('在新裝置的標題畫面輸入，就能取回這個帳號（7 天內有效，用過一次就失效；這台裝置之後要用新的繼承碼才能再登入）。')}</p>
  {/if}
</Modal>

<style>
  .bg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .topshade {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 120px;
    background: linear-gradient(180deg, rgb(243 246 253 / 0.85), transparent);
  }

  /* 角色腳下的光圈，把人和背景接起來 */
  .floor {
    position: absolute;
    left: 140px;
    bottom: 10px;
    width: 520px;
    height: 90px;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(61 155 255 / 0.55), rgb(24 61 214 / 0.18) 60%, transparent);
    filter: blur(2px);
  }

  .leader {
    position: absolute;
    left: 120px;
    bottom: 0;
    width: 560px;
    height: 620px;
    padding: 0;
    border: 0;
    background: none;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    animation: rise var(--bb-dur-5) var(--bb-ease) both;
  }

  .leader img {
    max-width: 100%;
    max-height: 100%;
    filter: drop-shadow(0 14px 28px rgb(11 15 26 / 0.45));
    animation: breathe 4s ease-in-out infinite;
    transform-origin: 50% 100%;
  }

  /* 只有半身像時往下沉一點，讓胸口以下被畫面底部切掉，比較像站在場景裡 */
  .leader:not(.full) img {
    height: 600px;
    max-height: none;
    margin-bottom: -40px;
  }

  @keyframes rise {
    from {
      transform: translateY(calc(60px * var(--bb-motion)));
      opacity: 0;
    }
  }

  @keyframes breathe {
    50% {
      transform: scaleY(calc(1 + 0.012 * var(--bb-motion)));
    }
  }

  .leader-name {
    position: absolute;
    left: 40px;
    bottom: 52px;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .leader-name b {
    padding: 4px 16px;
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    transform: skewX(var(--bb-skew));
  }

  .leader-name small {
    font-size: 11px;
    color: #fff;
    text-shadow: 0 1px 2px #000;
    padding-left: 10px;
  }

  .menu {
    position: absolute;
    right: 40px;
    top: 128px;
    width: 420px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .tile {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto auto;
    align-items: end;
    height: 112px;
    padding: 14px 28px;
    border: 0;
    text-align: left;
    clip-path: polygon(22px 0, 100% 0, calc(100% - 22px) 100%, 0 100%);
    box-shadow: inset 0 -4px 0 rgb(0 0 0 / 0.25);
    transition: transform var(--bb-dur-2) var(--bb-ease);
    animation: slide var(--bb-dur-4) var(--bb-ease) both;
    animation-delay: calc(var(--i) * 70ms);
  }

  .tile.big {
    height: 170px;
  }

  .tile:hover {
    transform: translateX(calc(-8px * var(--bb-motion)));
  }

  .tile:active {
    transform: scale(0.98);
  }

  @keyframes slide {
    from {
      transform: translateX(calc(80px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .t-blue {
    background: var(--bb-face-blue);
    color: #fff;
  }

  .t-yellow {
    background: var(--bb-face-yellow);
    color: var(--bb-on-yellow);
  }

  .t-ink {
    background: var(--bb-facets), linear-gradient(180deg, #232a40, #0b0f1a);
    color: #fff;
  }

  .tile .en {
    grid-column: 1 / 3;
    font: italic 400 22px/1 var(--bb-font-display);
    letter-spacing: 0.1em;
    opacity: 0.75;
    text-transform: uppercase;
  }

  .tile.big .en {
    font-size: 30px;
  }

  .tile b {
    font-size: 40px;
    font-weight: 900;
    line-height: 1.1;
    letter-spacing: 0.1em;
  }

  .tile.big b {
    font-size: 56px;
  }

  .tile small {
    grid-column: 1;
    font-size: 13px;
    opacity: 0.85;
  }

  .arrow {
    grid-column: 2;
    grid-row: 2 / 4;
    align-self: center;
    font-size: 26px;
  }

  .small {
    align-self: flex-end;
  }

  .daily {
    position: absolute;
    left: 40px;
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
