<script>
  // 畫面：title → home（大廳）→ stages（出擊）／gacha（招募）／roster（角色）；stages 開打後是 battle
  // 背景是 bluebolt-ui 的液態金屬（WebGL，不支援時退回漸層）＋ HUD 格線；切畫面時翻攪一下。
  import { HudBackdrop, Toaster, toast, setTheme } from 'bluebolt-ui';
  import { ChromeBackground, surge } from 'bluebolt-ui/fx';
  import { session, refresh } from './lib/api.svelte.js';
  import TitleScreen from './lib/components/TitleScreen.svelte';
  import HomeScreen from './lib/components/HomeScreen.svelte';
  import GachaScreen from './lib/components/GachaScreen.svelte';
  import RosterScreen from './lib/components/RosterScreen.svelte';
  import StageScreen from './lib/components/StageScreen.svelte';
  import BattleScreen from './lib/components/BattleScreen.svelte';

  setTheme('light');

  let screen = $state('title');
  let ticket = $state(null);

  function go(s) {
    if (s === screen) return;
    screen = s;
    surge();
  }

  async function enter() {
    try {
      await refresh();
      go('home');
    } catch {
      /* 錯誤訊息在 session.error，停在標題畫面 */
    }
  }

  const home = () => go('home');

  // 錯誤訊息用 bluebolt-ui 的通知顯示
  $effect(() => {
    if (!session.error) return;
    toast({ title: session.error, en: 'Error', tone: 'warn' });
    session.error = '';
  });

  // 1280×720 的舞台等比縮放到視窗
  let vw = $state(1280);
  let vh = $state(720);
  const scale = $derived(Math.min(vw / 1280, vh / 720));
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

<ChromeBackground amount={screen === 'battle' ? 0.25 : screen === 'title' ? 0.6 : 0.35} ribbons={0.8} />

<main class="stage" style:--stage-scale={scale}>
  <HudBackdrop fixed={false} label="VIRTUAL LEGENDS — M1" />
  {#key screen}
    {#if screen === 'title' || !session.profile}
      <TitleScreen onenter={enter} />
    {:else if screen === 'home'}
      <HomeScreen {go} />
    {:else if screen === 'gacha'}
      <GachaScreen onback={home} />
    {:else if screen === 'roster'}
      <RosterScreen onback={home} />
    {:else if screen === 'stages'}
      <StageScreen onback={home} onstart={(t) => ((ticket = t), go('battle'))} />
    {:else if screen === 'battle'}
      <BattleScreen {ticket} onexit={() => ((ticket = null), go('stages'))} />
    {/if}
  {/key}
  <Toaster />
</main>
