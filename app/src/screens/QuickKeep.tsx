import { Localized, useLocalization } from "@fluent/react";
import { useId } from "react";
import { Banner, Button, Checkbox } from "../components/kit";
import { Attention, Info } from "../icons";
import { fileName, formatWhen, type Conflict, type OfferedFile } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Heading, Name, QsStrip } from "./QsParts";

type Choice = NonNullable<Conflict["choice"]>;
const CHOICE_KEY: Record<Choice, string> = { "keep-both": "qs-keep-both", skip: "qs-skip" };
const CHOICES: Choice[] = ["keep-both", "skip"];

/**
 * Q10 Keep your existing work safe (receiver): a table of every offered file with what is already in
 * the folder and what will happen, a choice for each file that already has a namesake (keep both or
 * skip), the box approving the choices, and the note about files that changed. Replace is never
 * offered: the safety gate never overwrites a file that is already there. The main action stays off
 * until every clash has a choice and the person ticks their approval. A file that changed in the
 * folder since the person looked has no choice until they choose again.
 * Events: onChoose(path, choice), onApprovedChange, onContinue, onBack.
 */
export default function QuickKeep({
  files,
  conflicts,
  approved,
  onChoose,
  onApprovedChange,
  onContinue,
  onBack,
}: {
  files: OfferedFile[];
  conflicts: Conflict[];
  approved: boolean;
  onChoose: (path: string, choice: Choice) => void;
  onApprovedChange: (on: boolean) => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const language = typeof document === "undefined" ? "en" : document.documentElement.lang;
  const open = conflicts.some((c) => c.choice === null);
  const blocked = open || (conflicts.length > 0 && !approved);
  const byPath = new Map(conflicts.map((c) => [c.path, c]));
  return (
    <>
      <QsStrip onBack={onBack} receiving />
      <Heading id="qs-conflict-title" />
      <Localized id="qs-conflict-body">
        <p className="subhead" />
      </Localized>

      <div role="status" className="qs-status">
        {conflicts.length === 0 && (
          <Banner tone="ready">
            <Localized id="qs-no-conflicts">
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      <div className="qs-table-wrap">
        <table className="qs-table">
          <thead>
            <tr>
              <th scope="col">{l10n.getString("qs-conflict-col-file")}</th>
              <th scope="col">{l10n.getString("qs-conflict-col-existing")}</th>
              <th scope="col">{l10n.getString("qs-conflict-col-choice")}</th>
            </tr>
          </thead>
          <tbody>
            {files.map((f) => {
              const c = byPath.get(f.path);
              return (
                <tr key={f.path}>
                  <th scope="row">
                    <Name>{fileName(f.path)}</Name>
                  </th>
                  <td>
                    {c
                      ? c.existingModified
                        ? l10n.getString("qs-conflict-existing", { when: formatWhen(c.existingModified, language) })
                        : l10n.getString("qs-conflict-existing-unknown")
                      : l10n.getString("qs-conflict-none")}
                  </td>
                  <td>{c ? (c.choice ? l10n.getString(CHOICE_KEY[c.choice]) : l10n.getString("qs-conflict-choice-open")) : l10n.getString("qs-add-new")}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {conflicts.map((c) => (
        <ConflictChoice key={c.path} conflict={c} onChoose={onChoose} />
      ))}

      {conflicts.length > 0 && (
        <div className="qs-block qs-card">
          <Checkbox
            checked={approved}
            onChange={onApprovedChange}
            label={
              <Localized id="qs-conflict-approve">
                <span />
              </Localized>
            }
          />
        </div>
      )}

      {conflicts.length > 0 && (
        <div className="note">
          <Info />
          <div>
            <Localized id="qs-conflict-target-changed">
              <p className="note-text" />
            </Localized>
          </div>
        </div>
      )}

      <FrameFooter note="send">
        <Button variant="primary" disabled={blocked} aria-describedby="qs-conflict-why" onClick={onContinue}>
          <Localized id="qs-conflict-continue">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
      <p id="qs-conflict-why" className="qs-small">
        {blocked ? l10n.getString("qs-conflict-why") : ""}
      </p>
    </>
  );
}

function ConflictChoice({
  conflict: c,
  onChoose,
}: {
  conflict: Conflict;
  onChoose: (path: string, choice: Choice) => void;
}) {
  const { l10n } = useLocalization();
  const id = useId();
  const name = fileName(c.path);
  return (
    <div className="qs-field">
      <label htmlFor={id} className="qs-field-label">
        <Name>{name}</Name>
      </label>
      <select
        id={id}
        className="kit-select qs-select"
        value={c.choice ?? ""}
        onChange={(e) => {
          onChoose(c.path, e.target.value as Choice);
        }}
      >
        {c.choice === null && <option value="">{l10n.getString("qs-conflict-choice-open")}</option>}
        {CHOICES.map((choice) => (
          <option key={choice} value={choice}>
            {l10n.getString(CHOICE_KEY[choice])}
          </option>
        ))}
      </select>
      {c.changed && (
        <p className="qs-problem">
          <Attention />
          <span>{l10n.getString("qs-conflict-target-changed")}</span>
        </p>
      )}
      {c.choice === "keep-both" && <p className="qs-small">{l10n.getString("qs-keep-both-note", { newName: c.newName })}</p>}
    </div>
  );
}
