// 系列共通的角色資料：id、名字、簡介、配色沿用其他五款（VirtualQuest 的 src/lib/game/characters.js），
// 半身像（public/characters/<id>/<表情>.webp）是 90 年代版、麻將館那套衣服。
// 之後若搬進 VirtualCore，這份改成從那裡匯入。
const BASE = import.meta.env.BASE_URL;

const LIST = [
  { id: 'xiaocui', name: '小翠', intro: '綠色雙馬尾的元氣少女', color: '#3fbf86' },
  { id: 'yuehua', name: '月華', intro: '銀髮的冷靜大小姐', color: '#c9d4ff' },
  { id: 'aming', name: '阿明', intro: '熱血的刺蝟頭少年', color: '#f77f00' },
  { id: 'tangtang', name: '糖糖', intro: '愛撒嬌的粉紅包包頭', color: '#ff9ec7' },
  { id: 'xiaochun', name: '小春', intro: '橘色馬尾的運動少女', color: '#f8961e' },
  { id: 'lanjie', name: '蘭姐', intro: '戴眼鏡的麻將館老闆娘', color: '#9b7fe0' },
  { id: 'cangya', name: '蒼牙', intro: '銀髮狼耳、鼻子靈又好勝的狼族少年', color: '#5ab0ff' },
  { id: 'jinye', name: '金爺', intro: '鬍子帥氣、見過大風大浪的老牌賭客', color: '#e9b85e' },
  { id: 'leo', name: '雷歐', intro: '戴著王冠、一本正經的異國王子騎士', color: '#4fd1c5' },
  { id: 'alie', name: '阿烈', intro: '紅髮刀疤、豪爽愛冒險的傭兵', color: '#ff4d4d' },
];

export const CHARACTERS = new Map(LIST.map((c) => [c.id, c]));

/** 表情：normal happy win sad */
export const portrait = (id, expression = 'normal') => `${BASE}characters/${id}/${expression}.webp`;
