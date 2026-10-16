# Language card: French (fr)

## Tier and region
- Tier A: strongest model quality, most check coverage, many reference products. Standard metropolitan French that reads fine in Canada, Belgium, Switzerland and Africa. No regional slang.
- Direction: left to right. Script: Latin with accents. Fonts: system fonts are fine (Inter, Segoe UI, system-ui all cover it).

## Style and formality
- Formal "vous", always. Warm but plain, no marketing tone. Never "tu", never "on" for the app's own voice when "nous" is clearer.
- The person's gender is never needed: avoid sentences that agree with the user ("vous êtes connecté(e)"). Prefer the noun or the thing: "La connexion est établie." "Votre ancien ordinateur reste inchangé."
- Buttons: infinitive or short imperative ("Continuer", "Retour", "Annuler"). Titles in sentence case (only the first word capital).
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. Votre ancien ordinateur reste inchangé.
  2. Rien ne change sur votre ancien ordinateur.
  3. Votre ancien ordinateur n’est pas modifié.
- "laptop": "ordinateur" in running text, "ordinateur portable" in titles or where "old/new" needs the contrast. Final choice is the termbase.

## Plural categories (CLDR, verified with Intl.PluralRules): one, many, other
- Selector keys: `[one]`, `[many]`, `*[other]`. `one` covers 0, 1 and 1.5 ("0 fichier", "1 fichier"). `many` is exactly millions (1 000 000, 2 000 000 ...) and needs "de": "{ $count } de fichiers". `other` is everything else.
- Always write all three, even if `many` repeats `other` for a noun that does not take "de".

## Numbers, dates, digits
- Western digits. `Intl` (fr): 1 234 567,89 (narrow no-break space groups, comma decimal), 42 % (space before %), 12,5 Mo, "16 oct. 2026, 15:05" (24-hour).
- Units in the source (`MB/s`, `GB`) follow the French convention when written by hand: Mo, Go, Mo/s. Strings with `{ NUMBER() }` keep the placeable and change only the unit text.
- Pairing code: stays "482 917", digits only, readable and unchanged.

## Punctuation and quotes
- Guillemets « text » with a narrow no-break space (U+202F) inside. A no-break space (U+00A0) or U+202F before ? ! ; : and before %. Never let these wrap onto a new line alone.
- Apostrophe: curly ’ (l’ancien). Ellipsis: … (one character). Lists use no Oxford comma.
- Do not capitalise common nouns or months; capitalise only the first word and proper names.

## Brand names
- PCTwin, Wi-Fi, Windows, macOS stay as written; no article contraction problem: "sur Windows", "sur macOS".
- Elision before a brand: "de PCTwin" is correct; "d’OneDrive" elides before a vowel.

## Gender handling
- Nouns are gendered: every message must agree (adjectives, past participles). The laptop is masculine (un ordinateur), a file is masculine (fichier), a folder masculine (dossier), a connection feminine.
- Device labels: animal then colour, colour agrees with the animal. See `labels.md`.

## Avoid
- Anglicisms where a plain word exists ("synchroniser" is fine; "migrer", "backup" are not). Use "sauvegarde".
- Long compound titles; French runs 15-25% longer than English, so keep buttons short.
- Stacked negatives and legal tone in error messages.
