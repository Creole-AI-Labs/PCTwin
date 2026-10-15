import { Localized, useLocalization } from "@fluent/react";
import { useState } from "react";
import { Banner, Button, Checkbox } from "../components/kit";
import { Info } from "../icons";
import { totalBytes, useBytes, type OfferedFile, type SourceIssue } from "../quicksend";
import { FileRows, Heading, ISSUE_TEXT, QsActions, QsFooter, QsStrip, TallyBar } from "./QsParts";

/** Rows drawn before "Show all": a long folder stays a short screen. Every file is still sent; the count above is of all of them. */
const SHOWN = 100;

/**
 * Q03 Check what you are sharing (sender): the tally, the exact files that will be offered with their
 * sizes, the check box, and the things that are no longer as they were. The person's check is a
 * controlled value so the parent can clear it when the selection changes. The main action stays off
 * until it is ticked and nothing has an issue.
 * Events: onCheckedChange, onChange (back to the choice), onContinue, onBack.
 */
export default function QuickReview({
  files,
  issues = [],
  checked,
  onCheckedChange,
  onChange,
  onContinue,
  onBack,
}: {
  files: OfferedFile[];
  issues?: SourceIssue[];
  checked: boolean;
  onCheckedChange: (on: boolean) => void;
  onChange: () => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const [all, setAll] = useState(false);
  const blocked = !checked || issues.length > 0 || files.length === 0;
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-check-title" />
      <Localized id="qs-check-body">
        <p className="subhead" />
      </Localized>

      <TallyBar count={files.length} size={bytes(totalBytes(files))} />

      <div role="status" className="qs-status">
        {issues.length > 0 && (
          <Banner tone="attention">
            <ul className="qs-issues">
              {issues.map((i) => (
                <li key={i.path}>{l10n.getString(ISSUE_TEXT[i.kind], { name: i.path })}</li>
              ))}
            </ul>
          </Banner>
        )}
      </div>

      <FileRows files={files} limit={all ? undefined : SHOWN} included />
      {!all && files.length > SHOWN && (
        <Button
          variant="quiet"
          onClick={() => {
            setAll(true);
          }}
        >
          {l10n.getString("qs-check-show-all", { count: files.length })}
        </Button>
      )}

      <div className="qs-block qs-card">
        <Checkbox
          checked={checked}
          onChange={onCheckedChange}
          label={
            <Localized id="qs-check-ack">
              <span />
            </Localized>
          }
          hint={
            <Localized id="qs-check-ack-hint">
              <span />
            </Localized>
          }
        />
      </div>

      <QsActions>
        <button type="button" className="link" onClick={onChange}>
          <Localized id="qs-check-change">
            <span />
          </Localized>
        </button>
      </QsActions>

      <div className="note">
        <Info />
        <Localized id="qs-check-shared-note">
          <p className="note-text" />
        </Localized>
      </div>

      <QsFooter>
        <Button variant="primary" disabled={blocked} onClick={onContinue}>
          <Localized id="qs-check-continue">
            <span />
          </Localized>
        </Button>
      </QsFooter>
    </>
  );
}
