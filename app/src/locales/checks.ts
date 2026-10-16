/**
 * Translation gates for the screen text (Fluent files). Pure functions: no file or network access,
 * so the same code runs from the test (locales.test.ts) and from a script.
 *
 * Why the reference parser: the parser the app ships (@fluent/bundle) silently drops any message it
 * cannot read, so a broken translation would quietly show English. @fluent/syntax (Mozilla's
 * reference parser, dev-only) reports every broken entry, and that is what the syntax gate uses.
 *
 * Gates (each finding names its gate): file, syntax, extra-id, complete, structure, placeholders,
 * plural, markup, never-translate, length. The termbase soft check only ever warns.
 */
import {
  Junk,
  Message,
  NumberLiteral,
  Pattern,
  Placeable,
  SelectExpression,
  StringLiteral,
  Term,
  TermReference,
  TextElement,
  VariableReference,
  FunctionReference,
  MessageReference,
  parse,
  type Expression,
  type InlineExpression,
} from "@fluent/syntax";

export const TARGET_LOCALES = ["fr", "es", "pt", "ar", "hi", "zh-CN"] as const;
export type TargetLocale = (typeof TARGET_LOCALES)[number];

/** CLDR plural categories per language (also asserted against Intl.PluralRules in the test). */
export const PLURAL_CATEGORIES: Record<TargetLocale, readonly string[]> = {
  fr: ["one", "many", "other"],
  es: ["one", "many", "other"],
  pt: ["one", "many", "other"],
  ar: ["zero", "one", "two", "few", "many", "other"],
  hi: ["one", "other"],
  "zh-CN": ["other"],
};

const ALL_CATEGORIES = new Set(["zero", "one", "two", "few", "many", "other"]);

/** Ids a target may carry that English does not have (a gender helper for pairing labels). */
export const ALLOWED_EXTRA_PREFIXES: readonly string[] = ["label-gender-"];

/**
 * Pairing labels ("Blue Fox") may agree in gender in some languages. A target file then carries one
 * `label-gender-<animal>` message whose whole text is one of GENDER_KEYS, and the colour words
 * (`label-colour-*`) read it through a `{ $gender -> ... }` select that English does not have. The
 * select must have one default variant (the code passes an empty gender when the helper is absent).
 */
export const GENDER_HELPER_PREFIX = "label-gender-";
export const GENDER_SELECT_MESSAGE_PREFIX = "label-colour-";
export const GENDER_KEYS: readonly string[] = ["masculine", "feminine", "neuter"];

/**
 * Length ratios, in display-width units (see widthOf), target against English. Per language:
 *  soft/hard  for English text of 20 characters or more (warning above soft, failure above hard);
 *  shortSoft/shortHard for English text under 20 characters (buttons, labels), which legitimately
 *  expand more; a failure also needs the target to be more than 8 units longer than English, so
 *  "OK" becoming "D'accord" is not a failure.
 * Button-like text (English under 20 characters) also has an absolute width: warn over 26 units,
 * fail over 34, because a button must stay on one line in a 360 px window.
 * Provisional: set from typical expansion data (fr/es/pt about +20-35%, hi +20%, ar +0-10%,
 * zh-CN about 0.5-1.0 in width units) with headroom. The planted "good" French fixture measured 1.52
 * and 1.55 for ordinary sentences, so the Latin soft bar is 1.65. Re-tune after the first real
 * translations using the warnings the test prints.
 */
export const LENGTH_LIMITS: Record<TargetLocale, { soft: number; hard: number; shortSoft: number; shortHard: number }> = {
  fr: { soft: 1.65, hard: 1.9, shortSoft: 2.0, shortHard: 2.6 },
  es: { soft: 1.65, hard: 1.9, shortSoft: 2.0, shortHard: 2.6 },
  pt: { soft: 1.65, hard: 1.9, shortSoft: 2.0, shortHard: 2.6 },
  ar: { soft: 1.4, hard: 1.7, shortSoft: 1.8, shortHard: 2.2 },
  hi: { soft: 1.65, hard: 1.9, shortSoft: 2.0, shortHard: 2.5 },
  "zh-CN": { soft: 1.3, hard: 1.6, shortSoft: 1.5, shortHard: 2.0 },
};
export const SHORT_TEXT_CHARS = 20;
export const BUTTON_WIDTH_WARN = 26;
export const BUTTON_WIDTH_FAIL = 34;
export const TOO_SHORT_RATIO = 0.2; // warning only: a long English text with a tiny translation
export const SHORT_SLACK_UNITS = 8;

export interface Finding {
  gate: string;
  id: string;
  slot: string;
  reason: string;
}
export interface Report {
  failures: Finding[];
  warnings: Finding[];
}

export function emptyReport(): Report {
  return { failures: [], warnings: [] };
}
export function mergeReports(...reports: Report[]): Report {
  return { failures: reports.flatMap((r) => r.failures), warnings: reports.flatMap((r) => r.warnings) };
}
export function formatFinding(f: Finding): string {
  return `[${f.gate}] ${f.id}${f.slot ? f.slot : ""}: ${f.reason}`;
}

/* ---------------------------------------------------------------- parsing */

export interface SelectInfo {
  /** Selector as written, for messages: `$count` or `NUMBER($count)`. */
  selector: string;
  selectorVar: string | null;
  variants: { key: string; numeric: boolean; isDefault: boolean; usesSelectorVar: boolean }[];
  /** True when the English keys are plural categories or numbers (a count), not a word list. */
  plural: boolean;
}

export interface SlotInfo {
  /** "" for the message value, ".name" for an attribute. */
  slot: string;
  vars: Set<string>;
  /** Function calls with their named options, e.g. `NUMBER(maximumFractionDigits:0)`. */
  funcs: Set<string>;
  /** Message and term references, e.g. `-brand`, `msg-id`. */
  refs: Set<string>;
  selects: SelectInfo[];
  /** Every literal piece of text over all variants, joined by a line break (for markup and terms). */
  text: string;
  /** Rendering with the shortest / longest variant of each select, variables as "00". */
  shortest: string;
  longest: string;
}

export interface MessageInfo {
  id: string;
  slots: Map<string, SlotInfo>;
}

export interface Catalog {
  messages: Map<string, MessageInfo>;
  terms: string[];
  junk: string[];
  duplicates: string[];
  /** File hygiene problems: BOM, CRLF, missing final newline. */
  hygiene: string[];
}

function inlineName(e: InlineExpression): string | null {
  return e instanceof VariableReference ? e.id.name : null;
}

function selectorText(e: InlineExpression): { text: string; variable: string | null } {
  if (e instanceof VariableReference) return { text: `$${e.id.name}`, variable: e.id.name };
  if (e instanceof FunctionReference) {
    const v = e.arguments.positional.length > 0 ? inlineName(e.arguments.positional[0]) : null;
    return { text: `${e.id.name}(${v ? `$${v}` : "..."})`, variable: v };
  }
  return { text: e.type, variable: null };
}

/** Width of text in display units: wide (CJK) characters count 2, combining marks and joiners 0. */
export function widthOf(text: string): number {
  let w = 0;
  for (const ch of text) {
    const cp = ch.codePointAt(0) ?? 0;
    if (/[\p{M}\u200b-\u200f\u061c]/u.test(ch)) continue;
    const wide =
      (cp >= 0x1100 && cp <= 0x115f) ||
      (cp >= 0x2e80 && cp <= 0xa4cf) ||
      (cp >= 0xac00 && cp <= 0xd7a3) ||
      (cp >= 0xf900 && cp <= 0xfaff) ||
      (cp >= 0xfe30 && cp <= 0xfe6f) ||
      (cp >= 0xff00 && cp <= 0xff60) ||
      (cp >= 0xffe0 && cp <= 0xffe6) ||
      (cp >= 0x20000 && cp <= 0x3fffd);
    w += wide ? 2 : 1;
  }
  return w;
}

function analyseSlot(slot: string, pattern: Pattern): SlotInfo {
  const info: SlotInfo = {
    slot,
    vars: new Set(),
    funcs: new Set(),
    refs: new Set(),
    selects: [],
    text: "",
    shortest: "",
    longest: "",
  };
  const texts: string[] = [];

  const varsIn = (p: Pattern): Set<string> => {
    const out = new Set<string>();
    const walkP = (pp: Pattern): void => {
      for (const el of pp.elements) if (el instanceof Placeable) walkE(el.expression);
    };
    const walkE = (e: Expression): void => {
      if (e instanceof VariableReference) out.add(e.id.name);
      else if (e instanceof FunctionReference) {
        e.arguments.positional.forEach(walkE);
        e.arguments.named.forEach((n) => { walkE(n.value); });
      } else if (e instanceof SelectExpression) {
        walkE(e.selector);
        e.variants.forEach((v) => { walkP(v.value); });
      } else if (e instanceof Placeable) walkE(e.expression);
    };
    walkP(p);
    return out;
  };

  const expr = (e: Expression, pick: "short" | "long"): string => {
    if (e instanceof StringLiteral) return e.parse().value;
    if (e instanceof NumberLiteral) return e.value;
    if (e instanceof VariableReference) {
      info.vars.add(e.id.name);
      return "00";
    }
    if (e instanceof FunctionReference) {
      const named = e.arguments.named
        .map((n) => `${n.name.name}:${n.value instanceof StringLiteral ? n.value.parse().value : n.value.value}`)
        .sort()
        .join(",");
      info.funcs.add(`${e.id.name}(${named})`);
      e.arguments.positional.forEach((a) => expr(a, pick));
      return "00";
    }
    if (e instanceof MessageReference) {
      info.refs.add(e.attribute ? `${e.id.name}.${e.attribute.name}` : e.id.name);
      return "xxxxxx";
    }
    if (e instanceof TermReference) {
      info.refs.add(`-${e.id.name}${e.attribute ? `.${e.attribute.name}` : ""}`);
      return "xxxxxx";
    }
    if (e instanceof Placeable) return expr(e.expression, pick);
    if (e instanceof SelectExpression) {
      const sel = selectorText(e.selector);
      expr(e.selector, pick);
      const variants = e.variants.map((v) => {
        const key = v.key instanceof NumberLiteral ? v.key.value : v.key.name;
        return {
          key,
          numeric: v.key instanceof NumberLiteral,
          isDefault: v.default,
          usesSelectorVar: sel.variable !== null && varsIn(v.value).has(sel.variable),
        };
      });
      const keys = variants.map((v) => v.key);
      info.selects.push({
        selector: sel.text,
        selectorVar: sel.variable,
        variants,
        plural: keys.every((k) => ALL_CATEGORIES.has(k) || /^-?\d+(\.\d+)?$/.test(k)),
      });
      const rendered = e.variants.map((v) => render(v.value, pick));
      rendered.sort((a, b) => widthOf(a) - widthOf(b));
      return pick === "short" ? (rendered[0] ?? "") : (rendered[rendered.length - 1] ?? "");
    }
    return "";
  };

  const render = (p: Pattern, pick: "short" | "long"): string => {
    let out = "";
    for (const el of p.elements) {
      if (el instanceof TextElement) {
        out += el.value;
        texts.push(el.value);
      } else if (el instanceof Placeable) out += expr(el.expression, pick);
    }
    return out.replace(/\s+/g, " ").trim();
  };

  // Selects and variables are recorded on the first (short) pass; the second only renders.
  info.shortest = render(pattern, "short");
  const selectsSeen = info.selects.length;
  const textsSeen = texts.length;
  info.longest = render(pattern, "long");
  info.selects.length = selectsSeen; // the second pass would record every select twice
  texts.length = textsSeen;
  info.text = texts.join("\n");
  return info;
}

export function parseCatalog(source: string): Catalog {
  const hygiene: string[] = [];
  if (source.charCodeAt(0) === 0xfeff) hygiene.push("file starts with a byte-order mark (BOM)");
  if (source.includes("\r")) hygiene.push("file has CR or CRLF line endings (must be LF)");
  // eslint-disable-next-line no-control-regex
  if (/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\ufffd]/.test(source)) hygiene.push("file has a stray control or replacement character");
  if (source.length === 0) hygiene.push("file is empty");
  else if (!source.endsWith("\n")) hygiene.push("file does not end with a newline");

  const resource = parse((source.charCodeAt(0) === 0xfeff ? source.slice(1) : source), { withSpans: false });
  const messages = new Map<string, MessageInfo>();
  const terms: string[] = [];
  const junk: string[] = [];
  const duplicates: string[] = [];
  for (const entry of resource.body) {
    if (entry instanceof Junk) {
      const why = entry.annotations.map((a) => `${a.code}: ${a.message}`).join("; ");
      junk.push(`${why} | near: ${entry.content.trim().split("\n")[0]?.slice(0, 60) ?? ""}`);
    } else if (entry instanceof Term) {
      terms.push(entry.id.name);
    } else if (entry instanceof Message) {
      const id = entry.id.name;
      if (messages.has(id)) duplicates.push(id);
      const slots = new Map<string, SlotInfo>();
      if (entry.value) slots.set("", analyseSlot("", entry.value));
      for (const attr of entry.attributes) slots.set(`.${attr.id.name}`, analyseSlot(`.${attr.id.name}`, attr.value));
      messages.set(id, { id, slots });
    }
  }
  return { messages, terms, junk, duplicates, hygiene };
}

/* ---------------------------------------------------------------- termbase */

export interface Termbase {
  neverTranslate: string[];
  /** English term (with the English spellings to look for) -> approved forms per language code. */
  terms: { en: string; match: string[]; forms: Partial<Record<string, string[]>> }[];
  /** Words to avoid per language (technical or awkward renderings). */
  avoid: Record<string, string[]>;
}

type Json = Record<string, unknown>;
function asStrings(v: unknown): string[] {
  if (typeof v === "string") return [v];
  if (Array.isArray(v)) return v.flatMap(asStrings);
  return [];
}

/**
 * Reads the termbase JSON written by the Terminologist. Accepts `never_translate` as a list of
 * strings or of objects ({term}|{en}|{name}), and `terms` as a list of objects with an English key
 * (`en`|`term`|`source`) and either a `translations`/`forms` map or language codes as keys, or as a
 * map from English term to a per-language map.
 */
export function readTermbase(json: unknown): Termbase {
  const root = (json ?? {}) as Json;
  const never = asStrings(
    (Array.isArray(root["never_translate"]) ? root["never_translate"] : []).map((x: unknown) => {
      if (typeof x === "string") return x;
      const o = x as Json;
      return o["term"] ?? o["en"] ?? o["name"] ?? o["source"];
    }),
  );
  const terms: Termbase["terms"] = [];
  const raw = root["terms"];
  const addTerm = (en: unknown, entry: unknown): void => {
    if (typeof en !== "string" || typeof entry !== "object" || entry === null) return;
    const e = entry as Json;
    const map = (e["translations"] ?? e["forms"] ?? e["locales"] ?? e) as Json;
    const forms: Record<string, string[]> = {};
    for (const [k, v] of Object.entries(map)) if ((TARGET_LOCALES as readonly string[]).includes(k)) forms[k] = asStrings(v);
    const match = asStrings(e["match"]);
    terms.push({ en, match: match.length > 0 ? match : [en], forms });
  };
  if (Array.isArray(raw)) {
    for (const t of raw) {
      const o = t as Json;
      addTerm(o["en"] ?? o["term"] ?? o["source"], o);
    }
  } else if (raw && typeof raw === "object") {
    for (const [en, entry] of Object.entries(raw as Json)) addTerm(en, entry);
  }
  const avoid: Record<string, string[]> = {};
  const rawAvoid = root["avoid"];
  if (rawAvoid && typeof rawAvoid === "object") for (const [k, v] of Object.entries(rawAvoid as Json)) avoid[k] = asStrings(v);
  return { neverTranslate: [...new Set(never)].filter((s) => s.length > 0), terms, avoid };
}

function containsWord(haystack: string, needle: string): boolean {
  const esc = needle.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`(?<![\\p{L}\\p{N}])${esc}(?![\\p{L}\\p{N}])`, "u").test(haystack);
}

/* ---------------------------------------------------------------- gates */

export function checkSyntax(catalog: Catalog, label = "file"): Report {
  const r = emptyReport();
  catalog.hygiene.forEach((h) => r.failures.push({ gate: "file", id: label, slot: "", reason: h }));
  catalog.junk.forEach((j) => r.failures.push({ gate: "syntax", id: label, slot: "", reason: `does not parse: ${j}` }));
  catalog.duplicates.forEach((d) => r.failures.push({ gate: "syntax", id: d, slot: "", reason: "message id defined twice" }));
  return r;
}

export function checkIds(en: Catalog, target: Catalog, allowMissing: readonly string[], allowedExtraPrefixes: readonly string[] = ALLOWED_EXTRA_PREFIXES): Report {
  const r = emptyReport();
  for (const id of target.messages.keys()) {
    if (!en.messages.has(id) && !allowedExtraPrefixes.some((p) => id.startsWith(p)))
      r.failures.push({ gate: "extra-id", id, slot: "", reason: "id is not in the English source" });
  }
  for (const [id, m] of target.messages) {
    if (!id.startsWith(GENDER_HELPER_PREFIX) || en.messages.has(id)) continue;
    const value = m.slots.get("");
    const animal = id.slice(GENDER_HELPER_PREFIX.length);
    if (m.slots.size !== 1 || !value || value.vars.size > 0 || value.selects.length > 0 || !GENDER_KEYS.includes(value.shortest))
      r.failures.push({ gate: "gender-helper", id, slot: "", reason: `must be exactly one of ${GENDER_KEYS.join(", ")}` });
    if (!en.messages.has(`label-animal-${animal}`)) r.failures.push({ gate: "gender-helper", id, slot: "", reason: `no English message label-animal-${animal}` });
  }
  for (const t of target.terms) r.failures.push({ gate: "extra-id", id: `-${t}`, slot: "", reason: "terms (-name) are not allowed in translations" });
  for (const id of en.messages.keys()) {
    if (!target.messages.has(id) && !allowMissing.includes(id)) r.failures.push({ gate: "complete", id, slot: "", reason: "no translation" });
  }
  return r;
}

const setDiff = (a: Set<string>, b: Set<string>): string[] => [...a].filter((x) => !b.has(x)).sort();

export function checkMessage(
  locale: TargetLocale,
  en: MessageInfo,
  tr: MessageInfo,
  termbase: Termbase | null,
): Report {
  const r = emptyReport();
  const fail = (gate: string, slot: string, reason: string): void => { r.failures.push({ gate, id: en.id, slot, reason }); };
  const warn = (gate: string, slot: string, reason: string): void => { r.warnings.push({ gate, id: en.id, slot, reason }); };

  // Structure: same value/attribute names.
  for (const s of en.slots.keys()) if (!tr.slots.has(s)) fail("structure", s, s === "" ? "message value is missing" : `attribute ${s} is missing`);
  for (const s of tr.slots.keys()) if (!en.slots.has(s)) fail("structure", s, s === "" ? "message has a value but English has none" : `attribute ${s} is not in English`);

  const categories = PLURAL_CATEGORIES[locale];
  const lim = LENGTH_LIMITS[locale];

  const genderOk = en.id.startsWith(GENDER_SELECT_MESSAGE_PREFIX);
  for (const [slot, e] of en.slots) {
    const tRaw = tr.slots.get(slot);
    if (!tRaw) continue;
    let t = tRaw;
    if (genderOk) {
      // The gender select is allowed here; check it on its own, then compare the rest with English.
      const genderSelects = tRaw.selects.filter((x) => x.selectorVar === "gender");
      for (const g of genderSelects) {
        if (g.variants.filter((v) => v.isDefault).length !== 1) fail("plural", slot, "gender select must have exactly one default variant");
        const bad = g.variants.filter((v) => !GENDER_KEYS.includes(v.key) && !v.isDefault).map((v) => v.key);
        if (bad.length > 0) fail("plural", slot, `gender select uses unknown key(s): ${bad.join(", ")} (use ${GENDER_KEYS.join("/")})`);
      }
      if (genderSelects.length > 0 || tRaw.vars.has("gender")) {
        const vars = new Set(tRaw.vars);
        if (!e.vars.has("gender")) vars.delete("gender");
        t = { ...tRaw, vars, selects: tRaw.selects.filter((x) => x.selectorVar !== "gender" || e.selects.some((y) => y.selectorVar === "gender")) };
      }
    }

    // Placeholders, functions, references.
    for (const [what, a, b] of [
      ["variable", e.vars, t.vars],
      ["function call", e.funcs, t.funcs],
      ["reference", e.refs, t.refs],
    ] as const) {
      const lost = setDiff(a, b);
      const added = setDiff(b, a);
      if (lost.length > 0) fail("placeholders", slot, `${what} missing from translation: ${lost.join(", ")}`);
      if (added.length > 0) fail("placeholders", slot, `${what} not in English: ${added.join(", ")}`);
    }

    // Selects: same number, in the same order.
    if (e.selects.length !== t.selects.length) {
      fail("plural", slot, `English has ${String(e.selects.length)} select expression(s), translation has ${String(t.selects.length)}`);
    } else {
      e.selects.forEach((es, i) => {
        const ts = t.selects[i];
        const tkeys = ts.variants.map((v) => v.key);
        const defaults = ts.variants.filter((v) => v.isDefault);
        if (defaults.length !== 1) fail("plural", slot, `select ${es.selector} must have exactly one default variant (has ${String(defaults.length)})`);
        if (new Set(tkeys).size !== tkeys.length) fail("plural", slot, `select ${es.selector} has a repeated variant key`);
        if (es.plural) {
          if (defaults[0] && defaults[0].key !== "other") fail("plural", slot, `default variant must be [other], is [${defaults[0].key}]`);
          const numericKeys = es.variants.filter((v) => v.numeric).map((v) => v.key);
          const want = new Set([...categories, ...numericKeys]);
          const have = new Set(tkeys);
          const missing = [...want].filter((k) => !have.has(k));
          const extra = [...have].filter((k) => !want.has(k));
          if (missing.length > 0) fail("plural", slot, `plural variants missing for ${locale}: ${missing.join(", ")} (needs ${[...want].join("/")})`);
          if (extra.length > 0) fail("plural", slot, `plural variants not used by ${locale}: ${extra.join(", ")}`);
          if (ts.selector !== es.selector) fail("placeholders", slot, `selector is ${ts.selector}, English has ${es.selector}`);
          // English variants that do not show the count (e.g. just "file" / "files") need no count either.
          const englishUses = (key: string): boolean =>
            (es.variants.find((x) => x.key === key) ?? es.variants.find((x) => x.isDefault))?.usesSelectorVar ?? false;
          for (const v of ts.variants) {
            if (!v.usesSelectorVar && ts.selectorVar && !v.numeric && englishUses(v.key)) {
              if (["one", "two", "zero"].includes(v.key)) warn("plural", slot, `variant [${v.key}] omits $${ts.selectorVar}: check the number is conveyed in words`);
              else fail("plural", slot, `variant [${v.key}] does not use $${ts.selectorVar}`);
            }
          }
        } else {
          const ek = es.variants.map((v) => v.key).sort().join(",");
          const tk = [...tkeys].sort().join(",");
          if (ek !== tk) fail("plural", slot, `select ${es.selector}: variant keys [${tk}] differ from English [${ek}]`);
          const eDefault = es.variants.find((v) => v.isDefault)?.key;
          if (defaults[0] && eDefault !== undefined && defaults[0].key !== eDefault) fail("plural", slot, `default variant is [${defaults[0].key}], English default is [${eDefault}]`);
          if (ts.selector !== es.selector) fail("placeholders", slot, `selector is ${ts.selector}, English has ${es.selector}`);
        }
      });
    }

    // Markup and entities.
    const tagRe = /<\/?[A-Za-z][^<>]*>|&[A-Za-z][A-Za-z0-9]*;|&#x?[0-9A-Fa-f]+;/g;
    const eTags = [...new Set(e.text.match(tagRe) ?? [])].sort();
    const tTagsAll = t.text.match(tagRe) ?? [];
    const tTags = [...new Set(tTagsAll)].sort();
    if (eTags.join("\u0000") !== tTags.join("\u0000")) fail("markup", slot, `tags/entities differ: English [${eTags.join(" ")}], translation [${tTags.join(" ")}]`);
    // Any stray angle bracket that is not part of a recognised tag, or unbalanced tags.
    const stack: string[] = [];
    for (const tag of tTagsAll) {
      const m = /^<(\/?)([A-Za-z][\w-]*)[^<>]*?(\/?)>$/.exec(tag);
      if (!m || m[3] === "/" || /^(br|hr|img)$/i.test(m[2])) continue;
      if (m[1] === "") stack.push(m[2].toLowerCase());
      else if (stack.pop() !== m[2].toLowerCase()) fail("markup", slot, `tag </${m[2]}> closes nothing or the wrong tag`);
    }
    if (stack.length > 0) fail("markup", slot, `tag <${stack.join(">, <")}> is never closed`);
    // Angle brackets that are not inside a recognised tag.
    const brackets = (text: string, tags: string[]) => (text.match(/[<>]/g) ?? []).length - tags.join("").split("").filter((c) => c === "<" || c === ">").length;
    if (brackets(t.text, tTagsAll) > 0 && brackets(e.text, e.text.match(tagRe) ?? []) <= 0) fail("markup", slot, "stray < or > in translation (broken tag?)");
  }

  // Never-translate terms (per message, over all of its text).
  if (termbase) {
    const enAll = [...en.slots.values()].map((s) => s.text).join("\n");
    const trAll = [...tr.slots.values()].map((s) => s.text).join("\n");
    for (const term of termbase.neverTranslate) {
      if (containsWord(enAll, term) && !trAll.includes(term)) fail("never-translate", "", `"${term}" must stay exactly as written but is not in the translation`);
    }
    // Soft: approved forms.
    for (const term of termbase.terms) {
      const forms = term.forms[locale];
      if (!forms || forms.length === 0 || !term.match.some((m) => containsWord(enAll.toLowerCase(), m.toLowerCase()))) continue;
      if (!forms.some((f) => trAll.toLowerCase().includes(f.toLowerCase()))) warn("termbase", "", `"${term.en}" is not rendered with an approved form (${forms.slice(0, 3).join(" | ")})`);
    }
    for (const word of termbase.avoid[locale] ?? []) {
      if (trAll.toLowerCase().includes(word.toLowerCase())) warn("termbase", "", `uses "${word}", which the termbase says to avoid`);
    }
  }

  // Length (display width of the shortest rendering, per slot).
  for (const [slot, e] of en.slots) {
    const t = tr.slots.get(slot);
    if (!t) continue;
    const ew = widthOf(e.shortest);
    const tw = widthOf(t.shortest);
    if (ew === 0) continue;
    const ratio = tw / ew;
    const short = Array.from(e.shortest).length < SHORT_TEXT_CHARS;
    const hard = short ? lim.shortHard : lim.hard;
    const soft = short ? lim.shortSoft : lim.soft;
    const label = `${String(tw)} vs ${String(ew)} units, ratio ${ratio.toFixed(2)}`;
    if (ratio > hard && tw > ew + SHORT_SLACK_UNITS) fail("length", slot, `too long: ${label} (limit ${String(hard)}${short ? ", short text" : ""}): ${t.shortest}`);
    else if (ratio > soft && tw > ew + SHORT_SLACK_UNITS) warn("length", slot, `long: ${label} (soft limit ${String(soft)}): ${t.shortest}`);
    if (short) {
      if (tw > BUTTON_WIDTH_FAIL) fail("length", slot, `short text is ${String(tw)} units wide (button limit ${String(BUTTON_WIDTH_FAIL)}): ${t.shortest}`);
      else if (tw > BUTTON_WIDTH_WARN) warn("length", slot, `short text is ${String(tw)} units wide (warn over ${String(BUTTON_WIDTH_WARN)}): ${t.shortest}`);
    }
    if (ew >= 30 && ratio < TOO_SHORT_RATIO) warn("length", slot, `suspiciously short: ${label}: ${t.shortest}`);
  }
  return r;
}

export interface CheckOptions {
  locale: TargetLocale;
  allowMissing: readonly string[];
  termbase: Termbase | null;
  allowedExtraPrefixes?: readonly string[];
}

/** Every gate over one target catalog. `label` names the file in file-level findings. */
export function checkLocale(en: Catalog, target: Catalog, opts: CheckOptions, label: string = opts.locale): Report {
  const reports = [checkSyntax(target, label), checkIds(en, target, opts.allowMissing, opts.allowedExtraPrefixes)];
  for (const [id, e] of en.messages) {
    const t = target.messages.get(id);
    if (t) reports.push(checkMessage(opts.locale, e, t, opts.termbase));
  }
  return mergeReports(...reports);
}

/** Facts about the English source (Gate 0), for the report and the English-only tests. */
export function describeSource(en: Catalog): {
  messages: number;
  slots: number;
  withVariables: number;
  withFunctions: number;
  withSelects: number;
  pluralSelects: number;
  nonPluralSelects: number;
  withAttributes: number;
  withMarkup: number;
  maxWidth: { id: string; width: number };
} {
  let slots = 0, withVariables = 0, withFunctions = 0, withSelects = 0, pluralSelects = 0, nonPluralSelects = 0, withAttributes = 0, withMarkup = 0;
  let maxWidth = { id: "", width: 0 };
  for (const m of en.messages.values()) {
    let vars = false, funcs = false, sels = false, markup = false;
    if ([...m.slots.keys()].some((s) => s !== "")) withAttributes++;
    for (const s of m.slots.values()) {
      slots++;
      vars ||= s.vars.size > 0;
      funcs ||= s.funcs.size > 0;
      sels ||= s.selects.length > 0;
      markup ||= /<\/?[A-Za-z]|&[A-Za-z#]/.test(s.text);
      s.selects.forEach((x) => { if (x.plural) pluralSelects++; else nonPluralSelects++; });
      const w = widthOf(s.longest);
      if (w > maxWidth.width) maxWidth = { id: m.id + s.slot, width: w };
    }
    if (vars) withVariables++;
    if (funcs) withFunctions++;
    if (sels) withSelects++;
    if (markup) withMarkup++;
  }
  return { messages: en.messages.size, slots, withVariables, withFunctions, withSelects, pluralSelects, nonPluralSelects, withAttributes, withMarkup, maxWidth };
}
