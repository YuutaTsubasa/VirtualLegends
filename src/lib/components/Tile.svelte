<script>
  // 一張牌氣：數字牌寫中文數字＋花色，字牌寫東南西北中發白
  let { tile, selected = false, disabled = false, size = 'md', onclick = undefined } = $props();

  const NUM = ['', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
  const HONOR = ['', '東', '南', '西', '北', '中', '發', '白'];
  const SUIT = { man: '萬', pin: '筒', sou: '條' };

  const label = $derived(tile.suit === 'honor' ? HONOR[tile.rank] : `${NUM[tile.rank]}${SUIT[tile.suit]}`);
  const honorColor = $derived(tile.suit !== 'honor' ? '' : tile.rank === 5 ? 'var(--man)' : tile.rank === 6 ? 'var(--sou)' : 'var(--honor)');
</script>

<!-- 沒有 onclick 時是純顯示（例如敵人的牌，放在別的按鈕裡），用 span 才不會變成巢狀按鈕 -->
<svelte:element
  this={onclick ? 'button' : 'span'}
  class="tile {size} suit-{tile.suit}"
  class:selected
  class:off={disabled}
  type={onclick ? 'button' : undefined}
  disabled={onclick ? disabled : undefined}
  onclick={onclick}
  aria-pressed={onclick ? selected : undefined}
  role={onclick ? undefined : 'img'}
  aria-label={label}
>
  {#if tile.suit === 'honor'}
    <span class="big" style:color={honorColor}>{HONOR[tile.rank]}</span>
  {:else}
    <span class="num">{NUM[tile.rank]}</span>
    <span class="suit">{SUIT[tile.suit]}</span>
  {/if}
</svelte:element>

<style>
  .tile {
    width: 6.4cqh;
    height: 8.6cqh;
    padding: 0;
    border: 0.25cqh solid #b9ab8a;
    border-radius: 0.8cqh;
    background: linear-gradient(180deg, #fffdf6 0%, var(--ivory) 70%, var(--ivory-deep) 100%);
    box-shadow: 0 0.6cqh 0 #c8b994, 0 0.9cqh 0.6cqh rgba(0, 0, 0, 0.35);
    color: var(--suit);
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    line-height: 1;
    font-weight: 800;
    transition: transform 0.12s;
  }

  .tile.sm {
    width: 4cqh;
    height: 5.4cqh;
    font-size: 0.65em;
    border-width: 0.18cqh;
    box-shadow: 0 0.35cqh 0 #c8b994;
  }

  .tile.selected {
    transform: translateY(-1.6cqh);
    border-color: var(--gold);
    outline: 0.3cqh solid var(--gold);
  }

  button.tile:disabled {
    opacity: 0.4;
  }

  .num {
    font-size: 1.15em;
    color: var(--ink);
  }

  .suit {
    margin-top: 0.4cqh;
    font-size: 1.1em;
  }

  .big {
    font-size: 1.6em;
  }
</style>
