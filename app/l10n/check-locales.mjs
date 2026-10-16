// Runs the translation gates from the command line (same code as src/locales/locales.test.ts).
//
//   node l10n/check-locales.mjs                    all six languages, failures and warnings
//   node l10n/check-locales.mjs fr ar              only these languages
//   node l10n/check-locales.mjs --gate0 OUT.json   also write a per-message inventory of the English
//                                                  source (placeholders, selects, attributes, length)
//   node l10n/check-locales.mjs --json OUT.json    write all findings as JSON
//
// Exit code 1 if any gate fails. Needs Node 22.6 or later (runs the TypeScript checks directly).
// Writes nothing unless --gate0 or --json is given.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { TARGET_LOCALES, checkLocale, describeSource, formatFinding, parseCatalog, readTermbase, checkSyntax } from "../src/locales/checks.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
let gate0Out = null;
let jsonOut = null;
const langs = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--gate0") gate0Out = args[++i];
  else if (args[i] === "--json") jsonOut = args[++i];
  else langs.push(args[i]);
}
const read = (p) => readFileSync(join(root, p), "utf8");
const en = parseCatalog(read("src/locales/en/main.ftl"));
const termbase = readTermbase(JSON.parse(read("l10n/termbase.json")));

let failed = false;
const all = {};
const enSyntax = checkSyntax(en, "en");
if (enSyntax.failures.length > 0) {
  failed = true;
  console.log(`en: ${enSyntax.failures.map(formatFinding).join("\n")}`);
}
console.log(`English: ${JSON.stringify(describeSource(en))}`);

if (gate0Out) {
  const inventory = [...en.messages.values()].map((m) => ({
    id: m.id,
    slots: [...m.slots.values()].map((s) => ({
      slot: s.slot || "value",
      variables: [...s.vars],
      functions: [...s.funcs],
      references: [...s.refs],
      selects: s.selects.map((x) => ({ selector: x.selector, plural: x.plural, variants: x.variants.map((v) => (v.isDefault ? "*" : "") + v.key) })),
      markup: /<\/?[A-Za-z]|&[A-Za-z#]/.test(s.text),
      length: s.shortest.length,
      text: s.shortest,
    })),
  }));
  writeFileSync(gate0Out, JSON.stringify(inventory, null, 1));
}

for (const locale of TARGET_LOCALES) {
  if (langs.length > 0 && !langs.includes(locale)) continue;
  const target = parseCatalog(read(`src/locales/${locale}/main.ftl`));
  const report = checkLocale(en, target, { locale, allowMissing: [], termbase });
  all[locale] = report;
  const byGate = {};
  for (const f of report.failures) byGate[f.gate] = (byGate[f.gate] ?? 0) + 1;
  const status = report.failures.length === 0 ? "PASS" : "FAIL";
  if (report.failures.length > 0) failed = true;
  console.log(`${locale}: ${status}  ${target.messages.size} messages, ${report.failures.length} failures ${JSON.stringify(byGate)}, ${report.warnings.length} warnings`);
  for (const f of report.failures.slice(0, 40)) console.log(`  ${formatFinding(f)}`);
  if (report.failures.length > 40) console.log(`  ... ${report.failures.length - 40} more (use --json)`);
}
if (jsonOut) writeFileSync(jsonOut, JSON.stringify(all, null, 1));
process.exit(failed ? 1 : 0);
