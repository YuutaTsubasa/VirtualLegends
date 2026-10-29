<script>
  import { Button, DisplayWord, KineticText } from 'bluebolt-ui';
  import { session, signUp, claim } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';
  import { portrait } from '../characters.js';

  let { onenter } = $props();
  let code = $state('');
  let claiming = $state(false);

  async function start() {
    try {
      if (!session.account) await signUp();
      onenter();
    } catch {
      /* 錯誤訊息已經在 session.error */
    }
  }

  async function submitCode(e) {
    e.preventDefault();
    try {
      await claim(code);
      onenter();
    } catch {
      /* 同上 */
    }
  }

  const LINEUP = ['yuehua', 'xiaocui', 'aming'];
</script>

<section class="title">
  <div class="lineup" aria-hidden="true">
    {#each LINEUP as id, i (id)}
      <img src={portrait(id, i === 1 ? 'happy' : 'normal')} alt="" style:--i={i} draggable="false" />
    {/each}
  </div>

  <div class="logo">
    <DisplayWord top="VIRTUAL" bottom="LEGENDS" size="132px" />
    <h1><KineticText text={t('虛擬新傳說')} delay={300} /></h1>
    <p class="tag">{t('Virtual 系列・回合制 RPG')}</p>
  </div>

  <div class="menu">
    <Button size="lg" en={session.account ? 'Continue' : 'Start'} onclick={start} disabled={session.busy}>
      {session.account ? t('繼續遊戲') : t('開始遊戲')}
    </Button>
    {#if claiming}
      <form onsubmit={submitCode}>
        <input bind:value={code} placeholder={t('12 位繼承碼')} maxlength="16" autocomplete="off" />
        <Button type="submit" variant="ink" disabled={session.busy || code.length < 12}>{t('取回帳號')}</Button>
      </form>
    {:else}
      <Button variant="ghost" shape="rect" size="sm" onclick={() => (claiming = true)}>{t('用繼承碼取回帳號')}</Button>
    {/if}
  </div>

  <small class="note">{t('第一版不收真錢；招募只用遊戲內的星屑，機率在招募畫面公開。')}</small>
</section>

<style>
  .title {
    position: absolute;
    inset: 0;
  }

  .lineup {
    position: absolute;
    right: -40px;
    bottom: 0;
    width: 760px;
    height: 640px;
  }

  .lineup img {
    position: absolute;
    bottom: 0;
    height: 430px;
    left: calc(var(--i) * 250px - 20px);
    filter: drop-shadow(0 10px 20px rgb(11 15 26 / 0.35));
    animation: rise var(--bb-dur-5) var(--bb-ease) both;
    animation-delay: calc(var(--i) * 90ms);
  }

  /* 中間那位站在最前面 */
  .lineup img:nth-child(2) {
    z-index: 2;
    height: 540px;
    left: 150px;
  }

  @keyframes rise {
    from {
      transform: translateY(calc(60px * var(--bb-motion)));
      opacity: 0;
    }
  }

  .logo {
    position: absolute;
    left: 60px;
    top: 70px;
    z-index: 3;
  }

  h1 {
    margin: 8px 0 0;
    font-size: 54px;
    font-weight: 900;
    letter-spacing: 0.18em;
    color: var(--bb-blue);
    -webkit-text-stroke: 1px var(--bb-ink);
  }

  .tag {
    display: inline-block;
    margin: 8px 0 0;
    padding: 4px 14px;
    background: var(--bb-yellow);
    color: var(--bb-on-yellow);
    font-weight: 800;
    transform: skewX(var(--bb-skew));
  }

  .menu {
    position: absolute;
    left: 60px;
    top: 470px;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
  }

  form {
    display: flex;
    gap: 8px;
  }

  input {
    font: 700 18px var(--bb-font-mono);
    letter-spacing: 0.15em;
    padding: 8px 12px;
    width: 230px;
    border: 2px solid var(--bb-ink);
    background: var(--bb-surface);
    color: var(--bb-ink);
    text-transform: uppercase;
  }

  .note {
    position: absolute;
    left: 60px;
    bottom: 34px;
    z-index: 3;
    font-size: 12px;
    color: var(--bb-ink-2);
  }
</style>
