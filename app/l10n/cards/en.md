# Language card: English (en), source language

Project: PCTwin screens. Source file: `app/src/locales/en/main.ftl`. All other cards translate from this.

## Tier and role
- Tier A (source). Never machine-translated; changes here version the string (Language Design, part L).
- Direction: left to right. Script: Latin. Region: neutral international English; British "Grey" and "Ladybird" are used in the label words.

## Voice
- Plain words a non-technical person can follow alone. Say what happened, then what to do next.
- Address: "you / your". Friendly, calm, never alarming. No jargon in the main flow (technical detail goes behind "more details").
- Reassurance pattern, always concrete: "Your old laptop stays unchanged." "Nothing moves until you approve the plan."
- Sentence case for titles and buttons. No exclamation marks. No emoji. No colour-only status.

## Plural categories (CLDR): one, other
- Fluent selector keys: `[one]` and `*[other]`. Numeric keys such as `[0]` are allowed and must be kept by every language.
- Use `{ NUMBER($count) -> ...}` as the source does; `$count` is always shown or implied.

## Numbers, dates, text
- Digits: Western (0-9). `Intl.NumberFormat('en')`: 1,234,567.89 ; 42% ; 12.5 MB ; times like "3:05 PM".
- Pairing code: Western digits, grouped "482 917", always shown left to right.
- Apostrophes and quotes are curly in the source: it’s, “text”.
- Brand and product names are never translated: PCTwin, Wi-Fi, Windows, macOS, OneDrive, WhatsApp (see `app/l10n/termbase.json`).

## Gender
- None. The label words are "Blue Fox": `label-picked = { $colour } { $animal }`.

## Avoid
- Words like "sync", "migrate", "payload", "daemon", "token" in the main flow.
- Promises not in the Product Spec (anything paid, any cloud claim on a free screen).
- Long labels: buttons stay under about 20 characters; screens must fit a 360 px window.
