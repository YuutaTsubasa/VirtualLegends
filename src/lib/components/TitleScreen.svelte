<script>
  // 標題：滿版主視覺＋標誌，點畫面任何地方開始（第一次會建立訪客帳號）。角落是繼承碼與版本。
  import { Button, Modal, DisplayWord } from 'bluebolt-ui';
  import { session, signUp, claim } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { costumeArt, spriteArt, sceneArt } from '../art.js';

  let { onenter } = $props();
  let code = $state('');
  let claiming = $state(false);

  async function start() {
    if (session.busy || claiming) return;
    try {
      if (!session.account) await signUp();
      onenter();
    } catch {
      /* 錯誤訊息已經在 session.error */
    }
  }

  async function submitCode(e) {
    e?.preventDefault();
    try {
      await claim(code);
      claiming = false;
      onenter();
    } catch {
      /* 同上 */
    }
  }

  const bg = sceneArt('title_bg');
  // 有戰鬥全身圖的角色站在主視覺前面（沒有就不站）
  const LINEUP = ['tangtang.daily', 'yuehua.moonlit', 'xiaocui.festival', 'aming.daily', 'xiaocui.daily'].map((id) => spriteArt(id)).filter(Boolean).slice(0, 3);
</script>

<svelte:window onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && start()} />

<button class="title" type="button" onclick={start} aria-label={t('開始遊戲')}>
  {#if bg}<img class="bg" src={bg} alt="" draggable="false" />{/if}
  <span class="shade" aria-hidden="true"></span>

  <span class="lineup" aria-hidden="true">
    {#each LINEUP as src, i (src)}
      <img {src} alt="" style:--i={i} draggable="false" />
    {/each}
  </span>

  <span class="logo">
    <DisplayWord top="VIRTUAL" bottom="LEGENDS" size="120px" />
    <span class="zh">{t('虛擬新傳說')}</span>
  </span>

  <span class="tap">{session.busy ? t('連線中…') : 'TAP TO START'}<small>{session.account ? t('點畫面繼續遊戲') : t('點畫面開始遊戲')}</small></span>
</button>

<div class="corner">
  <Button shape="rect" variant="ink" size="sm" onclick={() => (claiming = true)}>{t('繼承碼')}</Button>
</div>
<p class="ver">VIRTUAL LEGENDS — M1 ・ {t('不收真錢；招募機率公開')}</p>

<Modal bind:open={claiming} title={t('用繼承碼取回帳號')} sub="Transfer" width="520px">
  <form onsubmit={submitCode}>
    <input bind:value={code} placeholder={t('12 位繼承碼')} maxlength="16" autocomplete="off" />
  </form>
  {#snippet footer()}
    <Button shape="rect" variant="secondary" onclick={() => (claiming = false)}>{t('取消')}</Button>
    <Button shape="rect" disabled={session.busy || code.length < 12} onclick={submitCode}>{t('取回帳號')}</Button>
  {/snippet}
</Modal>

<style>
  .title {
    position: absolute;
    inset: 0;
    padding: 0;
    border: 0;
    background: none;
    overflow: hidden;
    cursor: pointer;
  }

  .bg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: drift 30s ease-in-out infinite alternate;
  }

  @keyframes drift {
    to {
      transform: scale(calc(1 + 0.06 * var(--bb-motion)));
    }
  }

  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, rgb(243 246 253 / 0.75) 0 26%, transparent 55%), linear-gradient(0deg, rgb(11 15 26 / 0.55), transparent 35%);
  }

  .lineup img {
    position: absolute;
    bottom: 26px;
    height: 520px;
    left: calc(560px + var(--i) * 230px);
    filter: drop-shadow(0 10px 24px rgb(11 15 26 / 0.45));
    animation: rise var(--bb-dur-5) var(--bb-ease) both;
    animation-delay: calc(200ms + var(--i) * 120ms);
  }

  .lineup img:nth-child(2) {
    z-index: 2;
    height: 600px;
    bottom: 10px;
  }

  @keyframes rise {
    from {
      transform: translateY(calc(80px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .logo {
    position: absolute;
    left: 56px;
    top: 64px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
  }

  .zh {
    margin-top: 6px;
    padding: 4px 22px;
    background: var(--bb-blue);
    color: #fff;
    font-size: 46px;
    font-weight: 900;
    letter-spacing: 0.2em;
    transform: skewX(var(--bb-skew));
    box-shadow: 8px 8px 0 var(--bb-yellow);
  }

  .tap {
    position: absolute;
    left: 0;
    right: 420px;
    bottom: 64px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    font: italic 400 44px var(--bb-font-display);
    letter-spacing: 0.12em;
    color: #fff;
    -webkit-text-stroke: 1.5px var(--bb-ink);
    paint-order: stroke fill;
    animation: blink 1.4s ease-in-out infinite;
    z-index: 3;
  }

  .tap small {
    font: 800 14px var(--bb-font-zh);
    letter-spacing: 0.3em;
    -webkit-text-stroke: 0;
    text-shadow: 0 1px 3px #000;
  }

  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }

  .corner {
    position: absolute;
    right: 24px;
    top: 20px;
  }

  .ver {
    position: absolute;
    left: 24px;
    bottom: 14px;
    margin: 0;
    font: 700 11px var(--bb-font-cond);
    letter-spacing: 0.15em;
    color: rgb(255 255 255 / 0.85);
    text-shadow: 0 1px 2px #000;
    pointer-events: none;
  }

  input {
    width: 100%;
    font: 700 24px var(--bb-font-mono);
    letter-spacing: 0.2em;
    padding: 10px 14px;
    border: 2px solid var(--bb-ink);
    background: var(--bb-surface);
    color: var(--bb-ink);
    text-transform: uppercase;
  }
</style>
