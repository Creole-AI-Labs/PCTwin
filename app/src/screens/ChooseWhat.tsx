import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Badge, Banner, Button, Checkbox, Row, RowList, Segmented } from "../components/kit";
import type { Choice, ChoiceView } from "../bridge";
import { formatBytes, pageLocale } from "../move";
import type { FolderRole } from "../plan";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";

export type ChooseMode = "everything" | "except" | "only";

/** The groups and apps that move for a mode and the person's ticks. */
export function choiceOf(view: ChoiceView, mode: ChooseMode, ticked: readonly FolderRole[], appsTicked: boolean): Choice {
  const all = view.groups.map((g) => g.role);
  switch (mode) {
    case "everything":
      return { groups: all, apps: view.apps > 0 };
    case "except":
      // Ticked means "leave behind".
      return { groups: all.filter((r) => !ticked.includes(r)), apps: view.apps > 0 && !appsTicked };
    case "only":
      // Ticked means "bring".
      return { groups: all.filter((r) => ticked.includes(r)), apps: view.apps > 0 && appsTicked };
  }
}

/**
 * The old laptop, after it has found everything: the person chooses what moves, here where their
 * things are (decided 28 October 2026). Everything; Everything except (the leave-behind list starts
 * with what PCTwin leaves behind anyway, such as caches); or Only what I choose. The choice only
 * leaves things out; the new laptop then shows what is coming and its person accepts it.
 */
export default function ChooseWhat({ view, onContinue }: { view: ChoiceView; onContinue: (choice: Choice) => void }) {
  const { l10n } = useLocalization();
  const [mode, setMode] = useState<ChooseMode>("everything");
  const [ticked, setTicked] = useState<FolderRole[]>([]);
  const [appsTicked, setAppsTicked] = useState(false);
  const pick = (m: ChooseMode) => {
    setMode(m);
    setTicked([]);
    setAppsTicked(false);
  };
  const choice = choiceOf(view, mode, ticked, appsTicked);
  const nothing = choice.groups.length === 0 && !choice.apps;
  const size = (files: number, bytes: number) => l10n.getString("choose-group-size", { files, size: formatBytes(bytes, pageLocale()) });
  const toggle = (role: FolderRole, on: boolean) => {
    setTicked((t) => (on ? [...t, role] : t.filter((r) => r !== role)));
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
          <ul className="mv-people">
            {view.groups.map((g) => (
              <li key={g.role}>
                <Checkbox
                  checked={ticked.includes(g.role)}
                  onChange={(on) => {
                    toggle(g.role, on);
                  }}
                  label={l10n.getString(`plan-role-${g.role}`)}
                  hint={size(g.files, g.bytes)}
                />
              </li>
            ))}
            {view.apps > 0 && (
              <li>
                <Checkbox
                  checked={appsTicked}
                  onChange={setAppsTicked}
                  label={l10n.getString("choose-apps")}
                  hint={l10n.getString("choose-apps-count", { count: view.apps })}
                />
              </li>
            )}
          </ul>
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
