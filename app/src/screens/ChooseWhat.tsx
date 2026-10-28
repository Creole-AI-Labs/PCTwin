import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Badge, Banner, Button, Checkbox, Row, RowList, Segmented } from "../components/kit";
import type { Choice, ChoiceFolder, ChoiceView } from "../bridge";
import { formatBytes, pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

export type ChooseMode = "everything" | "except" | "only";

/** A folder's key in the tree: its group and the folders inside it. */
export const keyOf = (f: Pick<ChoiceFolder, "role" | "parts">): string => JSON.stringify([f.role, ...f.parts]);

/** Whether `f` is `top` or inside it. */
function within(f: ChoiceFolder, top: ChoiceFolder): boolean {
  return f.role === top.role && f.parts.length >= top.parts.length && top.parts.every((p, i) => f.parts[i] === p);
}

/** `top` and every folder inside it. */
export function subtree(view: ChoiceView, top: ChoiceFolder): ChoiceFolder[] {
  return view.folders.filter((f) => within(f, top));
}

/** A folder's state from the ticks of everything in it: ticked, not, or some. */
export function stateOf(view: ChoiceView, top: ChoiceFolder, ticked: ReadonlySet<string>): "on" | "off" | "mixed" {
  const all = subtree(view, top);
  const on = all.filter((f) => ticked.has(keyOf(f))).length;
  return on === 0 ? "off" : on === all.length ? "on" : "mixed";
}

/** The folders that move for a mode and the person's ticks (a tick means "moves"). */
export function choiceOf(view: ChoiceView, mode: ChooseMode, ticked: ReadonlySet<string>, appsTicked: boolean): Choice {
  const folders = mode === "everything" ? view.folders : view.folders.filter((f) => ticked.has(keyOf(f)));
  return { folders: folders.map((f) => ({ role: f.role, parts: f.parts })), apps: view.apps > 0 && (mode === "everything" || appsTicked) };
}

/** Files and bytes in `top` and everything inside it. */
function totals(view: ChoiceView, top: ChoiceFolder): { files: number; bytes: number } {
  return subtree(view, top).reduce((t, f) => ({ files: t.files + f.files, bytes: t.bytes + f.bytes }), { files: 0, bytes: 0 });
}

/**
 * The old laptop, after it has found everything: the person chooses what moves, here where their
 * things are (decided 28 October 2026). Everything; Everything except (every folder starts ticked,
 * the person unticks what stays, and what PCTwin leaves behind anyway, such as caches, is listed);
 * or Only what I choose (nothing starts ticked). Each group opens into its folders, one level at a
 * time; a folder with some of its folders ticked shows as partly ticked. The choice only leaves
 * things out; the new laptop then shows what is coming and its person accepts it.
 */
export default function ChooseWhat({ view, onContinue }: { view: ChoiceView; onContinue: (choice: Choice) => void }) {
  const { l10n } = useLocalization();
  const [mode, setMode] = useState<ChooseMode>("everything");
  const [ticked, setTicked] = useState<ReadonlySet<string>>(() => new Set(view.folders.map(keyOf)));
  const [appsTicked, setAppsTicked] = useState(true);
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const pick = (m: ChooseMode) => {
    setMode(m);
    // Everything except starts with everything ticked; Only what I choose with nothing.
    setTicked(new Set(m === "only" ? [] : view.folders.map(keyOf)));
    setAppsTicked(m !== "only");
    setOpen(new Set());
  };
  const choice = choiceOf(view, mode, ticked, appsTicked);
  const nothing = choice.folders.length === 0 && !choice.apps;
  const size = (files: number, bytes: number) => l10n.getString("choose-group-size", { files, size: formatBytes(bytes, pageLocale()) });
  const toggle = (top: ChoiceFolder) => {
    const keys = subtree(view, top).map(keyOf);
    // Ticked turns off; not ticked or partly ticked turns all of it on.
    const on = stateOf(view, top, ticked) !== "on";
    setTicked((t) => {
      const next = new Set(t);
      for (const k of keys) {
        if (on) next.add(k);
        else next.delete(k);
      }
      return next;
    });
  };
  const children = (top: ChoiceFolder) => view.folders.filter((f) => f.parts.length === top.parts.length + 1 && within(f, top));
  const groupsTop = view.folders.filter((f) => f.parts.length === 0);
  const chosen = new Set(choice.folders.map(keyOf));
  const chosenTotal = view.folders.filter((f) => chosen.has(keyOf(f))).reduce((t, f) => ({ files: t.files + f.files, bytes: t.bytes + f.bytes }), { files: 0, bytes: 0 });

  const branch = (top: ChoiceFolder) => {
    const key = keyOf(top);
    const kids = children(top);
    const isOpen = open.has(key);
    const state = stateOf(view, top, ticked);
    const t = totals(view, top);
    const name = top.parts.length === 0 ? l10n.getString(`plan-role-${top.role}`) : top.parts[top.parts.length - 1];
    return (
      <li key={key} className="mv-tree-item">
        <div className="mv-tree-row">
          {kids.length > 0 ? (
            <Button
              variant="quiet"
              aria-expanded={isOpen}
              aria-label={l10n.getString(isOpen ? "choose-close" : "choose-open", { name })}
              onClick={() => {
                setOpen((o) => {
                  const next = new Set(o);
                  if (isOpen) next.delete(key);
                  else next.add(key);
                  return next;
                });
              }}
            >
              {isOpen ? "−" : "+"}
            </Button>
          ) : (
            <span className="mv-tree-spacer" aria-hidden="true" />
          )}
          <Checkbox
            checked={state === "on"}
            mixed={state === "mixed"}
            onChange={() => {
              toggle(top);
            }}
            label={<bdi dir="auto">{name}</bdi>}
            hint={size(t.files, t.bytes)}
          />
        </div>
        {isOpen && <ul className="mv-tree">{kids.map(branch)}</ul>}
      </li>
    );
  };

  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="choose-title" />
      <p className="subhead">{l10n.getString("choose-body")}</p>
      <Segmented
        label={l10n.getString("choose-mode")}
        value={mode}
        options={[
          { value: "everything", label: l10n.getString("choose-everything") },
          { value: "except", label: l10n.getString("choose-except") },
          { value: "only", label: l10n.getString("choose-only") },
        ]}
        onChange={pick}
      />
      {mode === "everything" ? (
        <RowList>
          {view.groups.map((g) => (
            <Row key={g.role} title={l10n.getString(`plan-role-${g.role}`)} detail={size(g.files, g.bytes)} />
          ))}
          {view.apps > 0 && <Row title={l10n.getString("choose-apps")} detail={l10n.getString("choose-apps-count", { count: view.apps })} />}
        </RowList>
      ) : (
        <>
          <p className="subhead">{l10n.getString(mode === "except" ? "choose-except-hint" : "choose-only-hint")}</p>
          <ul className="mv-tree">{groupsTop.map(branch)}</ul>
          {view.apps > 0 && (
            <ul className="mv-people">
              <li>
                <Checkbox checked={appsTicked} onChange={setAppsTicked} label={l10n.getString("choose-apps")} hint={l10n.getString("choose-apps-count", { count: view.apps })} />
              </li>
            </ul>
          )}
        </>
      )}
      {mode === "except" && (
        <>
          <p className="subhead">{l10n.getString("choose-left-title")}</p>
          <RowList>
            <Row title={l10n.getString("choose-left-caches")} end={<Badge>{l10n.getString("choose-left-badge")}</Badge>} />
            {view.leftBehind.map((l) => (
              <Row key={l.why} title={l10n.getString(`choose-left-${l.why}`)} detail={size(l.items, l.bytes)} end={<Badge>{l10n.getString("choose-left-badge")}</Badge>} />
            ))}
          </RowList>
        </>
      )}
      <p className="subhead" role="status">
        {l10n.getString("choose-total", { files: chosenTotal.files, size: formatBytes(chosenTotal.bytes, pageLocale()) })}
      </p>
      {nothing && (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("choose-nothing")}</Banner>
        </div>
      )}
      <FrameFooter>
        <Button
          variant="primary"
          disabled={nothing}
          onClick={() => {
            onContinue(choice);
          }}
        >
          {l10n.getString("choose-continue")}
        </Button>
      </FrameFooter>
    </>
  );
}
