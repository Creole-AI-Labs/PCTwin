# Language card: Spanish (es)

## Tier and region
- Tier A: strong model quality and many references. One neutral international Spanish for Spain and Latin America (Language Design, part J: one base, regional changes only where wording differs).
- Direction: left to right. Script: Latin with accents, ñ, ¿ ¡. System fonts cover it.

## Style and formality
- Informal "tú" (tu, ti, contigo), never "vosotros/vuestro", never "vos" forms, never "usted". Friendly and plain.
- Use neutral vocabulary understood in Mexico, Colombia, Argentina, Chile and Spain. Avoid regional slang and avoid "ordenador" or "computadora" alone: prefer "portátil" (understood everywhere) and let the termbase decide. "Archivo" (not "fichero"), "carpeta", "programa" or "aplicación" (not "app" when avoidable; keep "app" only if the termbase says so).
- Gender of the user is never needed: avoid "listo/lista", "conectado/conectada" about the person. Use "Todo está listo" or the thing as subject.
- Buttons: infinitive or imperative tú ("Continuar", "Atrás", "Cancelar"). Sentence case.
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. Tu portátil anterior no cambia.
  2. No se cambia nada en tu portátil anterior.
  3. Tu portátil anterior se queda como está.

## Plural categories (CLDR, verified): one, many, other
- Selector keys: `[one]`, `[many]`, `*[other]`. `one` is exactly 1 (so "0 archivos" uses `other`). `many` is millions (1 000 000): "{ $count } de archivos". Write all three; `many` takes "de".

## Numbers, dates, digits
- Western digits. `Intl` (es): 1.234.567,89 (dot groups, comma decimal), 42 % (space before %), 12,5 MB, "16 oct 2026, 15:05". Four-digit numbers are not grouped by the CLDR rule for es (1234, not 1.234): trust NUMBER(), do not type separators.
- Pairing code stays "482 917", digits only.

## Punctuation and quotes
- Opening and closing marks always: ¿Qué hacemos? ¡Listo! Never only the closing mark.
- Quotes: « text » (RAE preferred, reads fine everywhere). Apostrophe is rare; use ’ if needed. Ellipsis: …
- Lowercase days and months. Sentence case titles. No space before ? ! : ;

## Brand names
- PCTwin, Wi-Fi, Windows, macOS unchanged. "tu Windows", "en macOS": no article before the brand unless natural ("el Wi-Fi").

## Gender handling
- Agree adjectives and participles with the noun: "el archivo seleccionado", "la carpeta seleccionada". The user is never gendered.
- Device labels: animal then colour, colour agrees with animal. See `labels.md`.

## Avoid
- "vosotros" forms, "ordenador" as the only word, "coger", "vale" (Spain-only); "computadora" alone (Latin America-only).
- Gender-marked wording about the user; "sincronizar" is fine, "migrar" and "backup" are not (use "copia de seguridad").
- Spanish runs 20-25% longer than English; keep buttons short.
