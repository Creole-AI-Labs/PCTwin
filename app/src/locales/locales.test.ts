/**
 * Translation gates for the screen text. Code, not opinion: every check here is deterministic and
 * a failing string names the gate and the reason. The checks themselves live in checks.ts so they
 * can also be run from a script; this file wires them to the real catalogs and proves, against
 * deliberately bad fixtures (src/locales/__fixtures__), that each gate really fails.
 *
 * Reproduce:  pnpm test src/locales            (whole file)
 *             pnpm test src/locales -t fr      (one language)
 * Nothing is written to disk. Warnings are printed once at the end.
 *
 * Thresholds (length, soft/hard) are documented next to LENGTH_LIMITS in checks.ts.
 */
import { afterAll, describe, expect, it } from "vitest";
import {
  PLURAL_CATEGORIES,
  TARGET_LOCALES,
  checkIds,
  checkLocale,
  checkMessage,
  checkSyntax,
  describeSource,
  formatFinding,
  parseCatalog,
  readTermbase,
  widthOf,
  type Catalog,
  type MessageInfo,
  type Finding,
  type Report,
  type TargetLocale,
  type Termbase,
} from "./checks";

/** English ids that are deliberately left untranslated in every language. Explicitly empty. */
const INTENTIONALLY_UNTRANSLATED: readonly string[] = [];

/** Ids removed from the English source when Replace left Quick Send (Q10); they must stay gone. */
const REMOVED_IDS = ["qs-replace", "qs-replace-separate", "qs-replace-warning", "qs-replace-confirm", "qs-replace-confirm-hint"];

const FTL = import.meta.glob<string>("./*/main.ftl", { query: "?raw", import: "default", eager: true });
const FIXTURES = import.meta.glob<string>("./__fixtures__/*.ftl", { query: "?raw", import: "default", eager: true });
const TERMBASE_FILES = import.meta.glob<unknown>("../../l10n/termbase.json", { import: "default", eager: true });

const ftl = (code: string): string => FTL[`./${code}/main.ftl`] ?? "";
const fixture = (name: string): string => (FIXTURES[`./__fixtures__/${name}`] ?? "").replace(/\r\n/g, "\n");

const termbaseRaw: unknown = Object.values(TERMBASE_FILES)[0];
const termbase: Termbase | null = termbaseRaw === undefined ? null : readTermbase(termbaseRaw);

const en = parseCatalog(ftl("en"));

/** Cap long failure lists so a whole-language failure stays readable. */
function summarize(findings: Finding[], limit = 15): string {
  const shown = findings.slice(0, limit).map(formatFinding);
  const more = findings.length > limit ? `\n... and ${String(findings.length - limit)} more` : "";
  return `${String(findings.length)} failing:\n${shown.join("\n")}${more}`;
}
function pick(c: Catalog, id: string): MessageInfo {
  const m = c.messages.get(id);
  if (!m) throw new Error(`no message ${id}`);
  return m;
}
const gate = (r: Report, ...gates: string[]): Finding[] => r.failures.filter((f) => gates.includes(f.gate));

/* Warnings collected from the real languages, printed once at the end. */
const collectedWarnings = new Map<string, Finding[]>();
afterAll(() => {
  const lines: string[] = [];
  for (const [locale, list] of collectedWarnings) {
    if (list.length === 0) continue;
    const byGate = new Map<string, Finding[]>();
    list.forEach((f) => byGate.set(f.gate, [...(byGate.get(f.gate) ?? []), f]));
    lines.push(`${locale}: ${String(list.length)} warnings (${[...byGate].map(([g, l]) => `${g} ${String(l.length)}`).join(", ")})`);
    for (const [, l] of byGate) l.slice(0, 3).forEach((f) => lines.push(`   ${formatFinding(f).slice(0, 160)}`));
  }
  if (lines.length > 0) console.log(`\nTranslation warnings (not failures)\n${lines.join("\n")}\n`);
});

/* ------------------------------------------------------------ Gate 0: English */

describe("Gate 0: English source", () => {
  it("parses with the reference parser: no syntax errors, LF, no BOM, final newline", () => {
    const r = checkSyntax(en, "en");
    expect(r.failures.length, summarize(r.failures)).toBe(0);
    expect(en.terms).toEqual([]);
  });

  it("has the removed Quick Send Replace ids gone", () => {
    for (const id of REMOVED_IDS) expect(en.messages.has(id), id).toBe(false);
  });

  it("every select is either a plural count or a word list with one default", () => {
    for (const m of en.messages.values())
      for (const s of m.slots.values())
        for (const sel of s.selects) {
          expect(sel.variants.filter((v) => v.isDefault).length, `${m.id}: one default`).toBe(1);
          if (sel.plural) expect(sel.variants.find((v) => v.isDefault)?.key, `${m.id}: default is other`).toBe("other");
        }
  });

  it("summarises the catalog", () => {
    const d = describeSource(en);
    expect(d.messages).toBeGreaterThan(900);
    console.log(`English source: ${JSON.stringify(d)}`);
  });
});

describe("Termbase", () => {
  it("exists and lists never-translate terms including the product name", () => {
    expect(termbase, "app/l10n/termbase.json is missing").not.toBeNull();
    expect(termbase?.neverTranslate).toContain("PCTwin");
  });
});

describe("Plural categories", () => {
  it.each(TARGET_LOCALES)("%s matches CLDR (Intl.PluralRules) and the hard-coded list", (locale) => {
    const fromIntl = [...new Intl.PluralRules(locale).resolvedOptions().pluralCategories].sort();
    expect(fromIntl).toEqual([...PLURAL_CATEGORIES[locale]].sort());
  });
});

/* ------------------------------------------------------------ Gate 1: each language */

describe.each(TARGET_LOCALES)("Gate 1: %s", (locale: TargetLocale) => {
  const target: Catalog = parseCatalog(ftl(locale));
  const report = checkLocale(en, target, { locale, allowMissing: INTENTIONALLY_UNTRANSLATED, termbase });
  collectedWarnings.set(locale, report.warnings);

  it("file is valid Fluent, UTF-8 without BOM, LF, final newline", () => {
    const r = gate(report, "file", "syntax");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("has no id that English does not have (except label-gender-*)", () => {
    const r = gate(report, "extra-id");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("is complete: every English id is translated", () => {
    const r = gate(report, "complete");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("keeps the same values and attributes", () => {
    const r = gate(report, "structure");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("keeps the same placeholders, function calls and references", () => {
    const r = gate(report, "placeholders");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("uses exactly this language's plural categories, with [other] as default", () => {
    const r = gate(report, "plural");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("keeps tags and entities identical and balanced", () => {
    const r = gate(report, "markup");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("leaves never-translate terms untouched", () => {
    const r = gate(report, "never-translate");
    expect(r.length, summarize(r)).toBe(0);
  });
  it("stays within the length limits", () => {
    const r = gate(report, "length");
    expect(r.length, summarize(r)).toBe(0);
  });
});

/* ------------------------------------------------------------ meta: planted bad strings */

describe("Meta: every gate fails on a planted bad string", () => {
  const source = parseCatalog(fixture("source.ftl"));
  const tb: Termbase = {
    neverTranslate: ["PCTwin", "Wi-Fi"],
    terms: [{ en: "safe", match: ["safe"], forms: { fr: ["sécurité"] } }],
    avoid: {},
  };
  const opts = (locale: TargetLocale) => ({ locale, allowMissing: [] as string[], termbase: tb });

  it("the fixture source itself is clean", () => {
    expect(checkSyntax(source, "source").failures).toEqual([]);
  });

  it("a correct translation passes every gate with no warnings", () => {
    const good = checkLocale(source, parseCatalog(fixture("good-fr.ftl")), opts("fr"), "good-fr");
    expect(good.failures.map(formatFinding)).toEqual([]);
    expect(good.warnings.map(formatFinding)).toEqual([]);
  });

  const bad = checkLocale(source, parseCatalog(fixture("bad-fr.ftl")), opts("fr"), "bad-fr");
  const badZh = checkLocale(source, parseCatalog(fixture("bad-zh-CN.ftl")), { locale: "zh-CN", allowMissing: ["app-name", "greet", "sizes", "back", "photo-button", "bold-note", "fee-note", "wifi-note", "brand-note", "missing-one", "long-note", "label-pick", "label-colour-green", "label-colour-blue", "label-animal-fox"], termbase: tb }, "bad-zh-CN");

  // [report, message id + slot, gate, text the reason must contain]
  const planted: [string, Report, string, string, string][] = [
    ["renamed variable", bad, "greet", "placeholders", "name"],
    ["function options dropped", bad, "sizes", "placeholders", "NUMBER(maximumFractionDigits:1)"],
    ["variable lost in an attribute", bad, "photo-button.aria-label", "placeholders", "name"],
    ["plural category missing", bad, "files-found", "plural", "missing for fr: many"],
    ["plural default is not other", bad, "items-left", "plural", "default variant must be [other]"],
    ["non-plural default moved", bad, "label-pick", "plural", "English default is [other]"],
    ["unclosed tag", bad, "bold-note", "markup", "never closed"],
    ["tag text changed", bad, "bold-note", "markup", "differ"],
    ["entity changed", bad, "fee-note", "markup", "&amp;"],
    ["Wi-Fi translated", bad, "wifi-note", "never-translate", "Wi-Fi"],
    ["brand name altered", bad, "brand-note", "never-translate", "PCTwin"],
    ["missing id", bad, "missing-one", "complete", "no translation"],
    ["extra id", bad, "not-in-english", "extra-id", "not in the English source"],
    ["too long", bad, "long-note", "length", "too long"],
    ["gender helper has a bad value", bad, "label-gender-fox", "gender-helper", "must be exactly one of"],
    ["gender helper for an unknown animal", bad, "label-gender-unicorn", "gender-helper", "no English message label-animal-unicorn"],
    ["gender select without a default (syntax error, so the colour is also missing)", bad, "bad-fr", "syntax", "does not parse"],
    ["gender select without a default leaves the colour untranslated", bad, "label-colour-green", "complete", "no translation"],
    ["gender select with a key that is not a gender", bad, "label-colour-blue", "plural", "unknown key(s): male"],
    ["extra plural category for Chinese", badZh, "files-found", "plural", "not used by zh-CN: one"],
    ["variant forgets the count", badZh, "items-left", "plural", "does not use $count"],
  ];
  it.each(planted)("%s", (_name, rep, idAndSlot, gateName, text) => {
    const hit = rep.failures.find((f) => `${f.id}${f.slot}` === idAndSlot && f.gate === gateName && f.reason.includes(text));
    expect(hit, `expected [${gateName}] ${idAndSlot} containing "${text}" in:\n${rep.failures.map(formatFinding).join("\n")}`).toBeDefined();
  });

  it("the bad fixtures fail only where planted (no collateral failures)", () => {
    const ids = new Set(bad.failures.map((f) => f.id).filter((id) => id !== "bad-fr"));
    expect([...ids].sort()).toEqual(
      ["bold-note", "brand-note", "fee-note", "files-found", "greet", "items-left", "label-pick", "label-colour-blue", "label-colour-green", "label-gender-fox", "label-gender-unicorn", "long-note", "missing-one", "not-in-english", "photo-button", "sizes", "wifi-note"].sort(),
    );
  });

  it("a syntax error is caught by the reference parser (and the app's own parser would hide it)", () => {
    const r = checkSyntax(parseCatalog(fixture("bad-syntax.ftl")), "bad-syntax");
    expect(gate(r, "syntax").length).toBeGreaterThanOrEqual(2);
    expect(r.failures[0]?.reason).toContain("does not parse");
  });

  it.each([
    ["byte-order mark", "﻿back = Retour\n", "BOM"],
    ["CRLF line endings", "back = Retour\r\n", "CRLF"],
    ["no final newline", "back = Retour", "newline"],
  ])("file hygiene: %s", (_n, text, needle) => {
    const r = checkSyntax(parseCatalog(text), "x");
    expect(gate(r, "file").some((f) => f.reason.includes(needle))).toBe(true);
  });

  it("the completeness allow-list is honoured only when an id is listed", () => {
    const t = parseCatalog("app-name = PCTwin\n");
    expect(gate(checkIds(source, t, []), "complete").length).toBeGreaterThan(5);
    expect(gate(checkIds(source, t, source.messages.has("back") ? ["back"] : []), "complete").some((f) => f.id === "back")).toBe(false);
  });

  it("soft termbase check only warns", () => {
    const t = parseCatalog("brand-note = PCTwin garde vos fichiers au chaud.\n");
    const r = checkMessage("fr", pick(source, "brand-note"), pick(t, "brand-note"), tb);
    expect(r.failures).toEqual([]);
    expect(r.warnings.some((w) => w.gate === "termbase")).toBe(true);
  });

  it("an omitted count in a one variant only warns", () => {
    const t = parseCatalog("items-left =\n    { $count ->\n        [one] Un élément restant\n        [many] { $count } d’éléments restants\n       *[other] { $count } éléments restants\n    }\n");
    const r = checkMessage("fr", pick(source, "items-left"), pick(t, "items-left"), tb);
    expect(r.failures).toEqual([]);
    expect(r.warnings.some((w) => w.gate === "plural")).toBe(true);
  });

  it("width counts Chinese as wide and ignores combining marks", () => {
    expect(widthOf("abc")).toBe(3);
    expect(widthOf("返回")).toBe(4);
    expect(widthOf("कि")).toBe(1);
  });
});
