<script>
  // 上方列：返回、標題、貨幣
  import { session } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';

  let { title, onback = undefined } = $props();
</script>

<header>
  {#if onback}
    <button class="back" type="button" onclick={onback} aria-label={t('返回')}>◀ {t('返回')}</button>
  {/if}
  <h2>{title}</h2>
  {#if session.profile}
    <ul class="money">
      <li title={t('星屑：招募用')}><span class="ic star">✦</span>{session.profile.stardust.toLocaleString()}</li>
      <li title={t('金幣：升級、突破用')}><span class="ic gold">●</span>{session.profile.gold.toLocaleString()}</li>
      <li title={t('練習筆記：升級用')}><span class="ic note">▤</span>{session.profile.notes.toLocaleString()}</li>
    </ul>
  {/if}
</header>

<style>
  header {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 9cqh;
    display: flex;
    align-items: center;
    gap: 2cqh;
    padding: 0 3cqh;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.55), rgba(0, 0, 0, 0));
    z-index: 5;
  }

  .back {
    background: rgba(0, 0, 0, 0.35);
    border: var(--line) solid var(--gold);
    border-radius: 0.8cqh;
    padding: 0.6cqh 1.8cqh;
  }

  h2 {
    margin: 0;
    font-size: 1.3em;
    letter-spacing: 0.15em;
    color: var(--gold);
    text-shadow: 0 0.3cqh 0 #000;
  }

  .money {
    margin: 0 0 0 auto;
    padding: 0;
    list-style: none;
    display: flex;
    gap: 1.6cqh;
  }

  .money li {
    background: rgba(0, 0, 0, 0.45);
    border: 0.2cqh solid rgba(227, 181, 82, 0.5);
    border-radius: 3cqh;
    padding: 0.5cqh 2cqh 0.5cqh 1cqh;
    font-variant-numeric: tabular-nums;
    min-width: 14cqh;
  }

  .ic {
    display: inline-block;
    width: 2.6cqh;
    text-align: center;
    margin-right: 0.6cqh;
  }
  .star {
    color: #9fe0ff;
  }
  .gold {
    color: var(--gold);
  }
  .note {
    color: #f3d9a4;
  }
</style>
