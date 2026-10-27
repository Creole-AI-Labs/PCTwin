/**
 * The engine's own examples of every shape that crosses the bridge (written by the shell's test,
 * src-tauri/src/contract.rs, into this folder) must each be read by bridge.ts exactly as sent.
 * Every .json file here is found and tested, so a new shape cannot be left out: a file with no
 * parse function fails. Regenerate the files with
 *   PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract
 */
import { describe, expect, it } from "vitest";
import {
  OPTIONAL_KEYS,
  parseConnectionView,
  parseEndedEarly,
  parseEvidence,
  parseHeadsUp,
  parseInterrupted,
  parseReadyNote,
  parseLabelView,
  parseMoveOutcome,
  parseMoveProblem,
  parseMoveProgress,
  parseMoveStage,
  parseNearbyView,
  parseNewView,
  parseOldView,
  parsePeopleHere,
  parsePeopleToPlace,
  parsePlanView,
  parsePrivacy,
  parseFound,
  parseHistoryEntry,
  parsePicked,
  parseOffered,
  parsePeer,
  parsePlaceAnswer,
  parseQsFolder,
  parseQsIncoming,
  parseQsProblem,
  parseQsProgress,
  parseQsReceipt,
  parseProblemView,
  parseReportItem,
  parseTidied,
} from "../bridge";
import { parseAnswer, parseRefusal, parseSnapshot } from "../guard";

const FILES = import.meta.glob<unknown>("./*.json", { import: "default", eager: true });

/** Each file's parse function, by file name. */
const PARSERS: Partial<Record<string, (value: unknown) => unknown>> = {
  "label-view": parseLabelView,
  "old-view": parseOldView,
  "new-view": parseNewView,
  "problem-view": parseProblemView,
  "nearby-view": parseNearbyView,
  evidence: parseEvidence,
  "heads-up": parseHeadsUp,
  "move-problem": parseMoveProblem,
  "move-link": parseConnectionView,
  "move-stage": parseMoveStage,
  "ended-early": parseEndedEarly,
  "plan-view": parsePlanView,
  "move-progress": parseMoveProgress,
  "move-outcome": parseMoveOutcome,
  "report-item": parseReportItem,
  interrupted: parseInterrupted,
  "ready-note": parseReadyNote,
  "people-here": parsePeopleHere,
  "people-to-place": parsePeopleToPlace,
  privacy: parsePrivacy,
  found: parseFound,
  "history-entry": parseHistoryEntry,
  "qs-picked": parsePicked,
  "qs-offered": parseOffered,
  "qs-incoming": parseQsIncoming,
  "qs-receipt": parseQsReceipt,
  "qs-progress": parseQsProgress,
  "qs-folder": parseQsFolder,
  "place-answer": parsePlaceAnswer,
  peer: parsePeer,
  "qs-problem": parseQsProblem,
  tidied: parseTidied,
  "guard-snapshot": parseSnapshot,
  "guard-refused": (v: unknown) => {
    const r = parseRefusal(v);
    return r === null ? null : { kind: r };
  },
  "guard-answer": (v: unknown) => {
    const a = parseAnswer(v);
    return a === null ? null : { kind: a };
  },
};

const families = Object.entries(FILES).map(([path, examples]) => [path.replace(/^\.\/|\.json$/g, ""), examples] as const);

describe("the engine's examples", () => {
  it("are all here", () => {
    expect(families.map(([name]) => name).sort()).toEqual(Object.keys(PARSERS).sort());
  });

  it.each(families)("%s: every example parses to exactly what was sent", (name, examples) => {
    const parse = PARSERS[name];
    expect(parse, `no parse function in bridge.ts for ${name}.json`).toBeDefined();
    expect(Array.isArray(examples)).toBe(true);
    const list = examples as unknown[];
    expect(list.length).toBeGreaterThan(0);
    for (const example of list) {
      const parsed = parse?.(example);
      expect(parsed, JSON.stringify(example)).not.toBeNull();
      expect(parsed).toEqual(example);
    }
  });

  it.each(families)("%s: a broken copy of each example is refused", (name, examples) => {
    const parse = PARSERS[name] ?? (() => null);
    const optional = OPTIONAL_KEYS[name] ?? [];
    for (const example of examples as Record<string, unknown>[]) {
      // An extra field, a missing field and a field of the wrong type are each refused. A field the
      // engine leaves out when it has nothing to say may be missing (whether it must be there for
      // this example is the parser's own rule, tested below).
      expect(parse({ ...example, extra: 1 }), "extra field").toBeNull();
      for (const key of Object.keys(example)) {
        const missing = Object.fromEntries(Object.entries(example).filter(([k]) => k !== key));
        if (!optional.includes(key)) expect(parse(missing), `without ${key}`).toBeNull();
        expect(parse({ ...example, [key]: { wrong: true } }), `${key} of the wrong type`).toBeNull();
      }
    }
  });
});

/** The screens' words in every launch language. */
const LOCALES = import.meta.glob<string>("../locales/*/main.ftl", { query: "?raw", import: "default", eager: true });
const SEVEN_LANGUAGES = ["ar", "en", "es", "fr", "hi", "pt", "zh-CN"];

describe("every problem the engine can send has words", () => {
  it("in all seven languages", () => {
    const languages = Object.keys(LOCALES)
      .map((path) => path.split("/")[2])
      .sort();
    expect(languages).toEqual(SEVEN_LANGUAGES);
    const problems = FILES["./problem-view.json"] as { key: string }[] | undefined;
    expect(problems?.length).toBeGreaterThan(0);
    const missing: string[] = [];
    for (const [path, text] of Object.entries(LOCALES)) {
      for (const { key } of problems ?? []) {
        // A Fluent message: the key at the start of a line, then "=".
        if (!new RegExp(`^${key} *=`, "m").test(text)) missing.push(`${path}: ${key}`);
      }
    }
    expect(missing).toEqual([]);
  });
});

describe("shapes the engine never sends are refused", () => {
  it.each([
    ["an unknown kind", parseOldView, { kind: "hacked" }],
    ["a kind in the wrong case", parseOldView, { kind: "Paired" }],
    ["a code without its countdown", parseOldView, { kind: "code", code: "123456" }],
    ["a negative countdown", parseOldView, { kind: "wait", secondsLeft: -1 }],
    ["a fractional countdown", parseOldView, { kind: "wait", secondsLeft: 1.5 }],
    ["two numbers to choose from", parseOldView, { kind: "choose", round: 1, numbers: [1, 2] }],
    ["a number too big for the screen", parseOldView, { kind: "choose", round: 1, numbers: [1, 2, 256] }],
    ["a label with markup for a key", parseLabelView, { kind: "picked", colour: "<b>blue</b>", animal: "fox" }],
    ["an unknown new-laptop kind", parseNewView, { kind: "pwned" }],
    ["a problem without networks", parseProblemView, { key: "pairing-failed" }],
    ["a problem with a network that is not text", parseProblemView, { key: "pairing-failed", networks: [1] }],
    ["an empty problem key", parseProblemView, { key: "", networks: [] }],
    ["a check from somewhere else", parseEvidence, { from: "cloud" }],
    ["memory without a reason", parseEvidence, { from: "memory" }],
    ["memory with an unknown reason", parseEvidence, { from: "memory", why: "because" }],
    ["the drive with a reason", parseEvidence, { from: "drive", why: "no-way" }],
    ["null", parseOldView, null],
    ["a string", parseNewView, "paired"],
    ["an array", parseEvidence, [{ from: "drive" }]],
    ["a copied line without its check", parseReportItem, { id: "1", name: "a", source: "a", status: "copied" }],
    ["a failed line that claims a check", parseReportItem, { id: "1", name: "a", source: "a", status: "failed", reason: "unreadable", evidence: { from: "drive" } }],
    ["a failed line with a reason of another status", parseReportItem, { id: "1", name: "a", source: "a", status: "failed", reason: "sign-in" }],
    ["a report note the shell has not", parseReportItem, { id: "1", name: "a", source: "a", status: "skipped-by-user", note: "moved-by-magic" }],
    ["a plan fingerprint that is not hex", parsePlanView, { revision: 1, fingerprint: "zz", groups: [], leftOut: [], preview: null }],
    ["a pause state the engine has not", parseMoveProgress, { state: "stopped", laptops: { old: null }, groups: [], counts: {}, bytes: { checked: 0, total: 0 } }],
    ["an unknown move problem", parseMoveProblem, { kind: "hacked" }],
  ] as const)("%s", (_name, parse, value) => {
    expect((parse as (v: unknown) => unknown)(value)).toBeNull();
  });
});
