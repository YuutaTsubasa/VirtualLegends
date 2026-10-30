// 本作自己的美術（Codex 畫的，挑過、轉成 webp 後放在 src/assets/art/）。Vite 打包時會掃這個資料夾，
// 有圖就用，沒有就退回系列共通的半身像，所以圖可以一張一張補。
//   costumes/<角色 id>/<表情>.webp   各版本的半身像（normal happy win sad）
//   sprites/<角色 id>/<姿勢>.webp    戰鬥用全身圖（idle attack）
//   enemies/<敵人 id>.webp
//   scenes/<名稱>.webp               標題、大廳、戰鬥背景
import { COSTUMES } from './data.js';
import { portrait } from './characters.js';

const FILES = import.meta.glob('../assets/art/**/*.webp', { eager: true, query: '?url', import: 'default' });
const art = (path) => FILES[`../assets/art/${path}.webp`];

/** 某個角色（版本）的半身像；這個版本沒有專屬圖就用系列角色的 */
export function costumeArt(costumeId, expression = 'normal') {
  return art(`costumes/${costumeId}/${expression}`) ?? art(`costumes/${costumeId}/normal`) ?? portrait(COSTUMES.get(costumeId).character, expression);
}

/** 戰鬥全身圖；沒有就回傳 undefined（畫面改用半身像） */
export const spriteArt = (costumeId, pose = 'idle') => art(`sprites/${costumeId}/${pose}`) ?? art(`sprites/${costumeId}/idle`);

export const enemyArt = (enemyId) => art(`enemies/${enemyId}`);

export const sceneArt = (name) => art(`scenes/${name}`);

/** 關卡用哪張戰鬥背景 */
export function stageScene(stageId) {
  const n = Number(stageId.split('-')[1]);
  return sceneArt(n >= 10 ? 'bg_rift' : n >= 6 ? 'bg_firewall' : 'bg_entrance');
}
