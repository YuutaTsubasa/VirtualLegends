<script>
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

  const LINEUP = ['yuehua', 'xiaocui', 'aming', 'tangtang', 'lanjie'];
</script>

<section class="title">
  <div class="lineup" aria-hidden="true">
    {#each LINEUP as id, i (id)}
      <img src={portrait(id, i === 1 ? 'happy' : 'normal')} alt="" style:--i={i} draggable="false" />
    {/each}
  </div>
  <div class="logo">
    <h1>{t('虛擬新傳說')}</h1>
    <p>VIRTUAL LEGENDS</p>
  </div>
  <div class="menu">
    <button class="btn" type="button" onclick={start} disabled={session.busy}>
      {session.account ? t('繼續遊戲') : t('開始遊戲')}
    </button>
    {#if claiming}
      <form onsubmit={submitCode}>
        <input bind:value={code} placeholder={t('12 位繼承碼')} maxlength="16" autocomplete="off" />
        <button class="btn sub" type="submit" disabled={session.busy || code.length < 12}>{t('取回帳號')}</button>
      </form>
    {:else}
      <button class="btn ghost" type="button" onclick={() => (claiming = true)}>{t('用繼承碼取回帳號')}</button>
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
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-end;
  }

  .lineup img {
    height: 78cqh;
    margin: 0 -7cqh;
    filter: drop-shadow(0 0 2cqh rgba(0, 0, 0, 0.6));
    opacity: 0.9;
    transform: translateY(calc((2 - var(--i)) * (2 - var(--i)) * 1.5cqh));
  }

  .logo {
    position: absolute;
    top: 7cqh;
    left: 0;
    right: 0;
    text-align: center;
  }

  h1 {
    margin: 0;
    font-size: 4.2em;
    letter-spacing: 0.25em;
    color: var(--ivory);
    -webkit-text-stroke: 0.5cqh var(--ink);
    paint-order: stroke fill;
    text-shadow: 0 1cqh 0 var(--accent-deep), 0 0 4cqh rgba(227, 181, 82, 0.7);
  }

  .logo p {
    margin: 0.6cqh 0 0;
    letter-spacing: 0.8em;
    color: var(--gold);
    font-weight: 700;
  }

  .menu {
    position: absolute;
    bottom: 12cqh;
    left: 0;
    right: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2cqh;
  }

  .menu .btn {
    min-width: 40cqh;
  }

  form {
    display: flex;
    gap: 1cqh;
  }

  input {
    font: inherit;
    width: 30cqh;
    padding: 1cqh 1.6cqh;
    border-radius: 1cqh;
    border: var(--line) solid var(--gold);
    background: var(--ivory);
    color: var(--ink);
    text-transform: uppercase;
    letter-spacing: 0.2em;
  }

  .note {
    position: absolute;
    bottom: 3cqh;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 0.7em;
    opacity: 0.8;
  }
</style>
