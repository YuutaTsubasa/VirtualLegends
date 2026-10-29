// 多國語言，做法跟其他五款一樣：原文（繁體中文）本身就是翻譯的 key。
//   t('招募')                     → 'Recruit'
//   t('{name} 加入了！', { name }) → '{name} joined!'
// 翻譯放在 src/lib/i18n/<語言>/*.js（各檔匯出一個 { 原文: 譯文 } 物件，會自動合併）。找不到翻譯就顯示原文。
// M1 只有中文完成；英、日的字典先就位，對外公開前補翻譯。
import { LANGUAGES, isLang, mergeDicts, loadLang, saveLang, translate, applyDocument } from 'virtual-core/i18n/lang.js';

const STORAGE_KEY = 'vl_lang';

export { LANGUAGES };

const DICTS = mergeDicts(import.meta.glob('./i18n/*/*.js', { eager: true }));

export const i18n = $state({ lang: loadLang(STORAGE_KEY) });

const apply = () => applyDocument(i18n.lang, t('虛擬新傳說'));

export function setLang(id) {
  if (!isLang(id)) return;
  i18n.lang = id;
  apply();
  saveLang(STORAGE_KEY, id);
}

/** 翻譯一段原文；{name} 之類的位置用 params 代入 */
export function t(text, params) {
  return translate(DICTS[i18n.lang], text, params);
}

apply();
