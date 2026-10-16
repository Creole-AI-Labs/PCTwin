# Language card: Chinese, Simplified (zh-CN)

## Tier and region
- Tier A: strong model quality for Simplified Chinese and a dedicated offline third opinion (Hunyuan-MT). Mainland standard (Putonghua) vocabulary. All Simplified variants map to this file; Traditional Chinese users see the "not yet available" note (Language Design, part J).
- Direction: left to right. Script: Han (Simplified). Fonts: Microsoft YaHei UI on Windows, PingFang SC on macOS; bundled fallback Noto Sans SC. `lang="zh-CN"` is set on the page (App.tsx sets `documentElement.lang`) so the right glyph variants load; keep it.

## Style and formality
- "你/你的" (friendly, short), not "您". Plain, short sentences. No classical or literary forms. Gender of the user never matters (no he/she).
- Vocabulary (mainland): 文件, 文件夹, 设置, 应用 (or 软件), 电脑 / 笔记本电脑, 账户, 下载, 连接. Not Taiwan terms (档案, 軟體, 資料夾 in Traditional).
- Buttons: verb or verb-object, 2-4 characters ("继续", "返回", "取消"). Avoid the technical word 迁移; the verb for "move" comes from the termbase.
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. 你的旧笔记本电脑保持原样。
  2. 旧笔记本电脑上不会有任何改动。
  3. 你的旧电脑不会被改动。
- "laptop": 笔记本电脑 at first mention or when contrast with "desktop" matters; 旧电脑 / 新电脑 elsewhere. Termbase decides.

## Plural categories (CLDR, verified): other
- The only key is `*[other]`. Write messages as a single `*[other]` variant (or as plain text). Numeric keys such as `[0]` in the source stay `[0]`. Measure words are required: 3 个文件, 2 台电脑, 5 项设置, 1 份报告.

## Numbers, dates, digits
- Western digits. `Intl` (zh-CN): 1,234,567.89 ; 42% ; 12.5 MB ; "2026年10月16日 15:05" (24-hour). Large numbers: 万 (10 000) for readability when hand-written, but trust NUMBER() output where a number is a placeable.
- Pairing code stays "482 917", digits only, unchanged. No space needed between Chinese and Latin or digits ("12.5 GB 的文件", "使用Wi-Fi"); pick one style and keep it: no spaces around Latin.

## Punctuation and quotes
- Full-width: ， 。 、 ； ： ？ ！ ； quotes “ 文本 ” and ‘ 文本 ’; ellipsis …… (two characters); dash ——. Parentheses （ ） full-width.
- No space after full-width punctuation. Never mix a Latin ", ." into Chinese sentences. Percent and unit stay half-width: 42%, MB.
- Line breaking: Chinese has no spaces; browsers break at any character. Add `line-break: strict` for the zh-CN pages so 。，？！ and closing quotes never start a line, and do not insert spaces to help wrapping.

## Brand names
- PCTwin, Wi-Fi, Windows, macOS stay in Latin letters, unchanged, with no spaces needed around them. Do not Chinese-ify product names.

## Gender handling
- None. Label words: colour then animal, no particle (蓝色狐狸). See `labels.md`.

## Avoid
- Traditional characters or Taiwan/HK terms; "您" on one screen and "你" on another; spaces between Chinese characters; English-style sentence punctuation; literal measure-word omissions.
- Text is usually much shorter than English (about 40-60%), so do not pad; but 12-14 px CJK text is dense, so keep the type size at least the English size.
