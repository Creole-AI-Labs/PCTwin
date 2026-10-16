// Pseudo-localization of the English screen text, for fit and hardcoded-string checks.
//
//   node l10n/pseudo.mjs OUT_DIR
//
// Writes OUT_DIR/pseudo.ftl (accented letters, vowels doubled for about +40% length, every text
// wrapped in [ ]) and OUT_DIR/pseudo-rtl.ftl (Latin letters replaced by Arabic letters, same
// elongation, wrapped in [ ]; used with dir="rtl"). Placeholders, functions, ids, attributes and
// plural structure are untouched because only text elements of the parsed file are rewritten.
// OUT_DIR must be outside the repository: nothing here is meant to be committed.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { FluentParser, FluentSerializer, TextElement, Transformer } from "@fluent/syntax";

const here = dirname(fileURLToPath(import.meta.url));
const out = process.argv[2];
if (!out) {
  console.error("usage: node l10n/pseudo.mjs OUT_DIR   (outside the repository)");
  process.exit(2);
}
if (resolve(out).startsWith(resolve(here, ".."))) {
  console.error("OUT_DIR must be outside the app folder");
  process.exit(2);
}

const ACCENT = {
  a: "á", b: "ƀ", c: "ç", d: "ð", e: "é", f: "ƒ", g: "ĝ", h: "ĥ", i: "í", j: "ĵ", k: "ķ", l: "ľ", m: "ɱ",
  n: "ñ", o: "ó", p: "þ", q: "ǫ", r: "ŕ", s: "š", t: "ţ", u: "ú", v: "ṽ", w: "ŵ", x: "ẋ", y: "ý", z: "ž",
  A: "Á", B: "Ɓ", C: "Ç", D: "Ð", E: "É", F: "Ƒ", G: "Ĝ", H: "Ĥ", I: "Í", J: "Ĵ", K: "Ķ", L: "Ľ", M: "Ṁ",
  N: "Ñ", O: "Ó", P: "Þ", Q: "Ǫ", R: "Ŕ", S: "Š", T: "Ţ", U: "Ú", V: "Ṽ", W: "Ŵ", X: "Ẋ", Y: "Ý", Z: "Ž",
};
const ARABIC = {
  a: "ا", b: "ب", c: "ج", d: "د", e: "ع", f: "ف", g: "غ", h: "ه", i: "ي", j: "ج", k: "ك", l: "ل", m: "م",
  n: "ن", o: "و", p: "ب", q: "ق", r: "ر", s: "س", t: "ت", u: "و", v: "ف", w: "و", x: "ك", y: "ي", z: "ز",
};
const VOWELS = new Set("aeiouyAEIOUY");

function pseudo(text, table, lower) {
  let res = "";
  for (const ch of text) {
    const m = table[lower ? ch.toLowerCase() : ch];
    res += m ?? ch;
    if (m && VOWELS.has(ch)) res += m;
  }
  return res;
}

class Pseudo extends Transformer {
  constructor(table, lower) {
    super();
    this.table = table;
    this.lower = lower;
  }
  visitTextElement(node) {
    node.value = pseudo(node.value, this.table, this.lower);
    return node;
  }
  visitPattern(node) {
    this.genericVisit(node);
    const els = node.elements;
    if (els[0] instanceof TextElement) els[0].value = "[" + els[0].value;
    else els.unshift(new TextElement("["));
    const last = els[els.length - 1];
    if (last instanceof TextElement) last.value = last.value.replace(/\s*$/, "") + "]";
    else els.push(new TextElement("]"));
    return node;
  }
}

const source = readFileSync(join(here, "..", "src", "locales", "en", "main.ftl"), "utf8");
mkdirSync(out, { recursive: true });
for (const [name, table, lower] of [["pseudo.ftl", ACCENT, false], ["pseudo-rtl.ftl", ARABIC, true]]) {
  const ast = new FluentParser({ withSpans: false }).parse(source);
  new Pseudo(table, lower).visit(ast);
  const text = new FluentSerializer({ withJunk: false }).serialize(ast);
  writeFileSync(join(out, name), text.endsWith("\n") ? text : text + "\n");
  console.log(`wrote ${join(out, name)} (${text.length} characters, English ${source.length})`);
}
