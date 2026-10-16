# Language card: Portuguese (pt), Brazilian base

## Tier and region
- Tier A: strong model quality, large audience. Brazilian Portuguese (pt-BR) is the one base; Portugal Portuguese falls back to it by CLDR parent (Language Design, part J). Use the 1990 spelling agreement forms.
- Direction: left to right. Script: Latin with accents, ç, ã, õ. System fonts cover it.

## Style and formality
- "você" (written with the accent), possessive "seu/sua". Warm, plain, conversational but not slangy. Never "tu", never "o senhor/a senhora".
- Avoid agreeing with the user's gender ("obrigado/obrigada", "conectado/conectada" about the person). Use the thing: "A conexão foi feita." "Tudo certo."
- Buttons: infinitive ("Continuar", "Voltar", "Cancelar"), sentence case.
- Vocabulary (Brazil): "arquivo", "pasta", "notebook" (also "laptop"), "computador", "aplicativo" (keep "app" only if the termbase says so), "celular". Not "ficheiro", "ecrã", "telemóvel", "utilizador" (use "usuário" or avoid).
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. Seu notebook antigo continua como está.
  2. Nada muda no seu notebook antigo.
  3. Seu notebook antigo não é alterado.

## Plural categories (CLDR, verified): one, many, other
- Selector keys: `[one]`, `[many]`, `*[other]`. `one` covers 0, 1 and 1.5 ("0 arquivo", "1 arquivo"). `many` is millions: "{ $count } de arquivos". Write all three; `many` takes "de".

## Numbers, dates, digits
- Western digits. `Intl` (pt): 1.234.567,89 (dot groups, comma decimal), 42% (no space before %), 12,5 MB, "16 de out. de 2026, 15:05". Dates are day month year.
- Pairing code stays "482 917", digits only.

## Punctuation and quotes
- Quotes: “ text ” (curly double, no inner spaces). Apostrophe is rare. Ellipsis: …
- Lowercase days and months. Sentence case titles. No space before ? ! : ;
- Contractions with brands are normal: "no Windows", "do Wi-Fi". For PCTwin itself avoid choosing an article: write "o PCTwin" consistently, or rephrase ("no aplicativo PCTwin"); never mix both.

## Brand names
- PCTwin, Wi-Fi, Windows, macOS unchanged. Avoid contractions that split a brand (no "d’OneDrive"; write "do OneDrive").

## Gender handling
- Agree adjectives and participles: "o arquivo selecionado", "a pasta selecionada". The user is never gendered.
- Device labels: animal then colour, colour agrees with animal. See `labels.md`.

## Avoid
- Portugal-only words (listed above) and the Portugal "a fazer" progressive; use the gerund ("fazendo").
- "sincronizar" is fine; "migrar" and "backup" are not (use "cópia de segurança" or "backup" only if the termbase allows).
- Portuguese runs 20-30% longer than English; keep buttons short.
