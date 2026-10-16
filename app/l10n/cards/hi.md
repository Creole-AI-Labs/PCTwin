# Language card: Hindi (hi)

## Tier and region
- Tier B: model quality is good for everyday text, but no published Hindi benchmark figures exist and Hindi has the fewest independent checks of the seven (Language Design, section 7). More flags expected; sensitive strings get the second pass. Standard Hindi (Devanagari), India-wide, everyday register.
- Direction: left to right. Script: Devanagari. Fonts: Nirmala UI on Windows, system Devanagari on macOS; bundled fallback Noto Sans Devanagari. Line height at least 1.5. No letter-spacing, no italic, no all-caps.

## Style and formality
- Polite "आप" with "आपका/आपकी/आपके" (agrees with the noun owned, not with the user). Imperatives in the polite "-एं" form: "चुनें", "जारी रखें", "वापस जाएं". Not "तुम", not "तू".
- Plain everyday Hindi. Common English loanwords in Devanagari are the natural choice: लैपटॉप, फ़ाइल, फ़ोल्डर, ऐप, सेटिंग, पासवर्ड, कंप्यूटर. Avoid heavy Sanskritised words (no "संगणक", "अभिलेख").
- The user's own gender is never needed: choose verbs that do not carry it ("आप चुन सकते हैं" carries m; use "चुनें" or "यह किया जा सकता है" or restructure).
- Reassurance, three phrasings of "Your old laptop stays unchanged":
  1. आपका पुराना लैपटॉप जैसा है वैसा ही रहता है।
  2. आपके पुराने लैपटॉप में कुछ नहीं बदलता।
  3. आपके पुराने लैपटॉप में कोई बदलाव नहीं किया जाता।

## Plural categories (CLDR, verified): one, other
- Selector keys: `[one]` and `*[other]`. `one` covers 0 and 1 ("0 फ़ाइल", "1 फ़ाइल"). `other` covers all else ("2 फ़ाइलें"). Numeric keys like `[0]` stay as in English.

## Numbers, dates, digits
- Western digits (numbering system latn) in the app: `Intl` (hi) gives 12,34,567.89 (lakh grouping: 3 digits then 2), 42%, 12.5 MB, "16 अक्टू॰ 2026, 3:05 pm". Never type Devanagari digits (०-९) by hand.
- Pairing code and match numbers: Western digits "482 917", readable, unchanged. Do not group by lakh.

## Punctuation and quotes
- Sentence end: danda "।" (not "."). Question: "?". Comma ",". Quotes “ text ”. Ellipsis: …
- Space before nothing; one space after danda and comma. Do not use a Latin full stop at the end of a Devanagari sentence.
- Use nukta consistently: फ़ाइल, ज़रूर, ज़्यादा, ख़त्म, वाई-फ़ाई (but keep the product spelling Wi-Fi as Latin per the termbase).

## Brand names
- PCTwin, Wi-Fi, Windows, macOS stay in Latin letters, unchanged, inside Devanagari sentences. Add case particles after a space: "Windows पर", "PCTwin में". Never hyphenate a brand into the Hindi word.

## Script handling
- Conjuncts (क्ष, त्र, ज्ञ, श्र, द्य) must not split: no `word-break: break-all`, no JS slicing by code unit; truncate with CSS ellipsis or Intl.Segmenter grapheme clusters.
- Matras and virama must stay attached; text length is about 10-25% longer than English, but taller. Check button heights.

## Gender handling
- Nouns are masculine or feminine and the verb, adjective and possessive agree: "मेरा लैपटॉप" (m), "मेरी फ़ाइल" (f), "मेरा फ़ोल्डर" (m), "कनेक्शन" (m). Common nouns: लैपटॉप m, फ़ाइल f, फ़ोल्डर m, सेटिंग f, ऐप m, कनेक्शन m.
- Device labels: colour (agrees) then animal, no animal-first order. See `labels.md`.

## Avoid
- Hinglish in Latin letters (write loanwords in Devanagari), English sentences with a few Hindi words, Urdu-origin heavy formal words, user-gendered verb forms ("चुन सकते हैं" is m, "सकती" is f).
- Missing nasal marks: नहीं, हैं, में, हम, कहें must carry the correct bindu or chandrabindu.
