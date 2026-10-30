<script>
  // 上方列：返回、畫面標題（外框大數字＋英文標籤＋中文標題）、貨幣
  import { Button } from 'bluebolt-ui';
  import { session } from '../api.svelte.js';
  import { t } from '../i18n.svelte.js';

  let { title, en = '', num = '', onback = undefined } = $props();
</script>

<header class="topbar">
  {#if onback}
    <Button shape="rect" variant="ink" size="sm" onclick={onback} aria-label={t('返回')}>« {t('返回')}</Button>
  {/if}
  <div class="ttl">
    {#if num}<span class="num" aria-hidden="true">{num}</span>{/if}
    <span class="txt">
      {#if en}<span class="en">{en}</span>{/if}
      <h2>{title}</h2>
    </span>
  </div>
  {#if session.profile}
    <ul class="money">
      <li title={t('星屑：招募用')}><i class="ic star">✦</i><b>{session.profile.stardust.toLocaleString()}</b></li>
      <li title={t('金幣：升級、突破用')}><i class="ic gold">●</i><b>{session.profile.gold.toLocaleString()}</b></li>
      <li title={t('練習筆記：升級用')}><i class="ic note">▤</i><b>{session.profile.notes.toLocaleString()}</b></li>
    </ul>
  {/if}
</header>

<style>
  .topbar {
    position: absolute;
    top: 18px;
    left: 40px;
    right: 40px;
    height: 76px;
    display: flex;
    align-items: center;
    gap: 20px;
    z-index: 5;
  }

  .ttl {
    display: flex;
    align-items: flex-end;
    gap: 18px;
  }

  .num {
    padding-right: 6px;
    font: italic 400 72px/0.8 var(--bb-font-display);
    color: transparent;
    -webkit-text-stroke: 2px var(--bb-ink);
  }

  .txt {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  .en {
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    font: italic 700 12px var(--bb-font-cond);
    letter-spacing: 0.2em;
    text-transform: uppercase;
    padding: 2px 10px;
    transform: skewX(var(--bb-skew));
    margin-left: 4px;
  }

  h2 {
    margin: 0;
    font-size: 34px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0.06em;
  }

  .money {
    margin: 0 0 0 auto;
    padding: 0;
    list-style: none;
    display: flex;
    gap: 8px;
  }

  .money li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 128px;
    padding: 6px 16px 6px 10px;
    background: var(--bb-strip);
    color: var(--bb-on-strip);
    clip-path: polygon(10px 0, 100% 0, calc(100% - 10px) 100%, 0 100%);
  }

  .money b {
    margin-left: auto;
    font: italic 400 20px var(--bb-font-display);
    letter-spacing: 0.04em;
  }

  .ic {
    font-style: normal;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    font-size: 13px;
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
  }

  .star {
    background: var(--bb-blue-2);
  }
  .gold {
    background: var(--bb-yellow);
    color: var(--bb-on-yellow);
  }
  .note {
    background: #fff;
    color: var(--bb-ink);
  }
</style>
