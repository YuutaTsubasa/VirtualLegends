<script>
  // 角色卡：bluebolt-ui 的 UnitCard＋本作的半身像、屬性、稀有度框（招募結果、角色一覽、編隊用）
  import { UnitCard as Card } from 'bluebolt-ui';
  import { COSTUMES, ELEMENT_NAME, characterOf } from '../data.js';
  import { portrait as portraitUrl } from '../characters.js';
  import { t } from '../i18n.svelte.js';

  let { costume, level = 1, selected = false, num = '', badge = '', onclick = undefined, width = 120 } = $props();

  const def = $derived(COSTUMES.get(costume));
  const ch = $derived(characterOf(costume));
</script>

<div class="unit rarity-{def.rarity} el-{def.element}" style:width="{width}px" style:--char={ch.color} class:static={!onclick}>
  <Card name={t(ch.name)} sub={t(def.name)} {level} stars={def.rarity} {num} {badge} {selected} role={t(ELEMENT_NAME[def.element])} onclick={onclick ?? (() => {})}>
    {#snippet portrait()}
      <img src={portraitUrl(ch.id)} alt="" draggable="false" />
    {/snippet}
  </Card>
</div>

<style>
  .unit {
    position: relative;
    flex: none;
  }

  .unit :global(.bb-unit) {
    background: radial-gradient(circle at 50% 30%, color-mix(in srgb, var(--char) 55%, white) 0%, var(--char) 50%, #141826 120%);
  }

  .unit :global(.art img) {
    object-position: 50% 10%;
  }

  /* 左上角的屬性字放在屬性色的菱形裡 */
  .unit :global(.role) {
    top: 6px;
    left: 6px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    font: 900 13px var(--bb-font-zh);
    background: var(--el);
    clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
    text-shadow: none;
  }

  /* 稀有度：底部細條 */
  .unit::after {
    content: '';
    position: absolute;
    left: 12px;
    right: 0;
    bottom: 0;
    height: 3px;
    background: var(--rarity);
    pointer-events: none;
  }

  .unit :global(.stars) {
    color: var(--rarity);
  }

  .static :global(.bb-unit) {
    cursor: default;
  }
</style>
