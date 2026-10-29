<script>
  // 角色卡：半身像＋稀有度＋屬性＋名稱（招募結果、角色一覽、編隊用）
  import { COSTUMES, SUIT_NAME, ROLE_NAME, characterOf } from '../data.js';
  import { portrait } from '../characters.js';
  import { t } from '../i18n.svelte.js';

  let { costume, level = null, selected = false, dim = false, badge = '', onclick = undefined } = $props();

  const def = $derived(COSTUMES.get(costume));
  const ch = $derived(characterOf(costume));
</script>

<button
  class="card rarity-{def.rarity} suit-{def.suit}"
  class:selected
  class:dim
  type="button"
  onclick={onclick}
  disabled={!onclick}
  style:--char={ch.color}
>
  <img src={portrait(ch.id)} alt="" draggable="false" />
  <span class="stars">{'★'.repeat(def.rarity)}</span>
  <span class="suit">{t(SUIT_NAME[def.suit])}</span>
  {#if badge}<span class="badge">{badge}</span>{/if}
  <span class="name">
    <b>{t(ch.name)}</b>
    <small>{t(def.name)}</small>
  </span>
  <span class="role">{t(ROLE_NAME[def.role])}{#if level !== null}・Lv {level}{/if}</span>
</button>

<style>
  .card {
    position: relative;
    width: 15cqh;
    height: 21cqh;
    padding: 0;
    border: 0.4cqh solid var(--rarity);
    border-radius: 1cqh;
    overflow: hidden;
    background: radial-gradient(circle at 50% 30%, color-mix(in srgb, var(--char) 55%, white) 0%, var(--char) 45%, #1a1a1a 120%);
    box-shadow: 0 0.6cqh 0 rgba(0, 0, 0, 0.4);
    color: var(--ivory);
    text-align: left;
    opacity: 1;
  }

  .card:disabled {
    cursor: default;
    opacity: 1;
  }

  .card.selected {
    outline: 0.5cqh solid var(--gold);
    transform: translateY(-0.8cqh);
  }

  .card.dim {
    filter: grayscale(0.8) brightness(0.6);
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: 50% 12%;
  }

  .stars {
    position: absolute;
    top: 0.6cqh;
    left: 0.8cqh;
    color: var(--rarity);
    font-size: 0.75em;
    text-shadow: 0 0 0.6cqh #000, 0 0 0.3cqh #000;
  }

  .suit {
    position: absolute;
    top: 0.5cqh;
    right: 0.6cqh;
    width: 3.4cqh;
    height: 3.4cqh;
    display: grid;
    place-items: center;
    background: var(--ivory);
    color: var(--suit);
    border-radius: 0.6cqh;
    font-weight: 800;
    font-size: 0.8em;
  }

  .badge {
    position: absolute;
    top: 5cqh;
    right: 0.4cqh;
    background: var(--accent);
    padding: 0.2cqh 0.8cqh;
    border-radius: 0.6cqh;
    font-size: 0.65em;
    font-weight: 700;
  }

  .name {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 2.6cqh;
    padding: 0.4cqh 0.8cqh;
    background: linear-gradient(0deg, rgba(0, 0, 0, 0.85), transparent);
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  .name small {
    font-size: 0.7em;
    color: var(--gold);
  }

  .role {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2.6cqh;
    padding: 0 0.8cqh;
    background: rgba(0, 0, 0, 0.85);
    font-size: 0.65em;
    display: flex;
    align-items: center;
  }
</style>
