<script>
  // 畫面：title → home（大廳）→ stages（出擊）／gacha（招募）／roster（角色）；stages 開打後是 battle
  import { session, refresh } from './lib/api.svelte.js';
  import TitleScreen from './lib/components/TitleScreen.svelte';
  import HomeScreen from './lib/components/HomeScreen.svelte';
  import GachaScreen from './lib/components/GachaScreen.svelte';
  import RosterScreen from './lib/components/RosterScreen.svelte';
  import StageScreen from './lib/components/StageScreen.svelte';
  import BattleScreen from './lib/components/BattleScreen.svelte';

  let screen = $state('title');
  let ticket = $state(null);

  async function enter() {
    try {
      await refresh();
      screen = 'home';
    } catch {
      /* 錯誤訊息在 session.error，停在標題畫面 */
    }
  }

  const home = () => (screen = 'home');

  // 錯誤訊息幾秒後自動消失
  $effect(() => {
    if (!session.error) return;
    const id = setTimeout(() => (session.error = ''), 4000);
    return () => clearTimeout(id);
  });
</script>

<main class="stage">
  {#if screen === 'title' || !session.profile}
    <TitleScreen onenter={enter} />
  {:else if screen === 'home'}
    <HomeScreen go={(s) => (screen = s)} />
  {:else if screen === 'gacha'}
    <GachaScreen onback={home} />
  {:else if screen === 'roster'}
    <RosterScreen onback={home} />
  {:else if screen === 'stages'}
    <StageScreen onback={home} onstart={(t) => ((ticket = t), (screen = 'battle'))} />
  {:else if screen === 'battle'}
    <BattleScreen {ticket} onexit={() => ((ticket = null), (screen = 'stages'))} />
  {/if}

  {#if session.error}
    <div class="error-toast" role="alert">{session.error}</div>
  {/if}
</main>
