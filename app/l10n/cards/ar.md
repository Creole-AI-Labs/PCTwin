# Language card: Arabic (ar)

## Tier and region
- Tier B: good model quality but 8-10 points behind European languages in published tests (Language Design, AI section), richer morphology, bidi risk. Expect more back-translation flags; sensitive strings get the second pass. Modern Standard Arabic (MSA), simple register, no dialect.
- Direction: right to left (`dir="rtl"` set by `directionOf("ar")`). Script: Arabic. Fonts: Segoe UI on Windows, system Arabic on macOS; bundled fallback Noto Sans Arabic. Line height about 1.6; never letter-space, never italic, no tatweel.

## Style and formality
- Polite, plain MSA. Address the user with the attached "ـك" (written the same for men and women: "جهازك", "ملفاتك"). That is the gender-neutral route: prefer nouns, "يمكنك", passive and impersonal forms. Where an imperative is unavoidable (buttons), use the standard masculine UI form or, better, a noun: "متابعة", "رجوع", "إلغاء", "اختيار".
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. جهازك القديم يبقى كما هو.
  2. لا يتغير أي شيء في الكمبيوتر المحمول القديم.
  3. لن يطرأ أي تغيير على جهازك القديم.
- "laptop": "الكمبيوتر المحمول" (clear, widely understood); "الجهاز" once the context is set. Not "حاسوب" alone. Termbase has the last word.

## Plural categories (CLDR, verified): zero, one, two, few, many, other
- Selector keys: `[zero]`, `[one]`, `[two]`, `[few]`, `[many]`, `*[other]`. Write all six.
- Ranges: zero = 0; one = 1; two = 2; few = 3-10 (and 103-110 ...); many = 11-99 (and 111-199 ...); other = 100-102, 200-202, decimals.
- Typical file forms: one "ملف واحد"; two "ملفان" (number may be dropped, brief allows); few "{ $count } ملفات"; many "{ $count } ملفًا"; other "{ $count } ملف".

## Numbers, dates, digits
- `Intl` with locale tag `ar`: Western digits (numbering system latn): 1,234,567.89 ; 42‎% ; times "3:05 م" with ص/م. Only region tags ar-EG or ar-SA give Arabic-Indic (٠-٩); the app uses the bare `ar` bundle, so do not tag it with a region.
- In the .ftl write digits as ASCII (0-9) only, never type Arabic-Indic digits by hand. The pairing code and match numbers are shown in Western digits, always left to right, grouped "482 917"; they must stay readable and the code box is `dir="ltr"`.
- Date order and ص/م: trust `Intl`. Do not hard-code.

## Punctuation and quotes
- Arabic comma ، semicolon ؛ question mark ؟ . Full stop is "." . Quotes « text ». Ellipsis: …
- No spaces inside « »; one space after ، and ؛. Do not mix ? with Arabic text.

## Mixed direction (what stays left to right inside the Arabic sentence)
- Paths, file names, drive letters (`C:\Users\Name`), network names, versions, codes (`PCT-7F3K-21`), "62 MB/s", brand names (PCTwin, Wi-Fi, Windows, macOS). Fluent isolates every `{ $variable }` (useIsolating is on): do not add direction marks.
- Typed static Latin words are fine but never glue a Latin brand to punctuation that precedes Arabic ("Wi-Fi: اختر"): reorder so Arabic follows the brand with a space.
- Put at most one Latin run next to each other. Do not start a sentence with a number or Latin token if you can avoid it.

## RTL checks the Locale pass makes on screens
- Mirror: Back/forward chevrons and arrows, "old laptop to new laptop" flow arrows, step trails, list disclosure arrows, undo/redo, send/receive direction arrows.
- Do not mirror: PCTwin logo, play/pause, clocks and spinners, check marks, Wi-Fi/signal bars, laptop drawings, "?" help glyphs, numerals.
- Progress bars fill from the right edge to the left; step 1 is at the right; "Back" sits at the start (right) and its arrow points right; "Next" points left.
- CSS uses logical start/end only (margin-inline-start, text-align: start); keyboard focus order follows reading order (right to left).
- Truncation (ellipsis) cuts at the end of the logical string, which is the left side; never cut inside a word.

## Gender handling
- Nouns and adjectives agree: laptop (الكمبيوتر) masculine; file (ملف) masculine; folder (مجلد) masculine; connection (اتصال) masculine; setting (إعداد) masculine; move/transfer (نقل) masculine. Device labels: noun then adjective that agrees; see `labels.md`.

## Avoid
- Dialect words, transliterated English where a plain MSA word exists, vowel marks (harakat) except to remove real ambiguity, Arabic-Indic digits.
- Long iDafa chains in buttons. Arabic runs shorter in characters but taller; check line wrapping in the 360 px window.
