import { Localized, useLocalization } from "@fluent/react";
import { useState } from "react";
import { Badge, Banner, Button } from "../components/kit";
import { Check, Lock } from "../icons";
import type { HelpCode } from "../people";
import { Eyebrow, Heading } from "./PairParts";
import { BackStrip } from "./PeopleParts";

type Props = {
  help: HelpCode;
  /** Whether this laptop has a system share sheet. Without one, only Copy is offered. */
  canShare?: boolean;
  onBack: () => void;
  /** Hand the code to the system share sheet (WhatsApp or any app), with the line to send. Nothing is claimed about what happens next. */
  onShare: (code: string, message: string) => void;
  onRetry: () => void;
};

type CopyState = { code: string; result: "copied" | "failed" } | null;

const NEVER = ["help-never-names", "help-never-files", "help-never-network"] as const;

/**
 * Get help, laid out as the mockup's S11: the code in its own card, then rows saying what it holds
 * and what it never holds, then the note. Copy says "copied" only when the system clipboard accepted
 * it; if it refuses, the person is told how to copy by hand. Share just hands the code on; it never
 * says it was sent.
 */
export default function GetHelp({ help, canShare = true, onBack, onShare, onRetry }: Props) {
  const { l10n } = useLocalization();
  const [copy, setCopy] = useState<CopyState>(null);

  if (help.status === "making") {
    return (
      <div className="help">
        <BackStrip id="pair-back" onBack={onBack} />
        <Eyebrow id="help-eyebrow" />
        <Heading key="making" id="help-title" />
        <div role="status">
          <Localized id="help-making">
            <p className="subhead" />
          </Localized>
        </div>
      </div>
    );
  }

  if (help.status === "failed") {
    return (
      <div className="help">
        <BackStrip id="pair-back" onBack={onBack} />
        <Eyebrow id="help-eyebrow" />
        <Heading key="failed" id="help-title" />
        <div className="people-notice">
          <Banner tone="attention">
            <Localized id="help-failed">
              <p />
            </Localized>
          </Banner>
        </div>
        <div className="people-actions">
          <Button variant="primary" onClick={onRetry}>
            {l10n.getString("help-try-again")}
          </Button>
        </div>
      </div>
    );
  }

  const { code } = help;
  // A result belongs to the code it was for; a new code starts uncopied.
  const result = copy?.code === code ? copy.result : null;

  const doCopy = async () => {
    try {
      // Guarded: some systems have no clipboard here, or refuse it.
      const clipboard = (navigator as { clipboard?: Clipboard }).clipboard;
      if (!clipboard) throw new Error("no clipboard");
      await clipboard.writeText(code);
      setCopy({ code, result: "copied" });
    } catch {
      setCopy({ code, result: "failed" });
    }
  };

  return (
    <div className="help">
      <BackStrip id="pair-back" onBack={onBack} />
      <Eyebrow id="help-eyebrow" />
      <Heading key="ready" id="help-title" />
      <Localized id="help-body">
        <p className="subhead" />
      </Localized>

      <div className="help-box">
        <Localized id="help-code-title">
          <p className="help-box-title" />
        </Localized>
        <Localized id="help-code" attrs={{ "aria-label": true }} vars={{ code }}>
          <p className="help-code" dir="ltr" translate="no" role="group">
            {code}
          </p>
        </Localized>
      </div>

      <ul className="help-rows">
        <li className="help-row">
          <span className="help-row-icon" aria-hidden="true">
            <Check />
          </span>
          <div className="help-row-copy">
            <Localized id="help-contains-title">
              <h2 />
            </Localized>
            <ul className="help-list">
              {help.included.map((kind) => (
                <Localized key={kind} id={`help-contains-${kind}`}>
                  <li />
                </Localized>
              ))}
            </ul>
          </div>
          <Badge icon={<Check />}>
            <Localized id="help-chip-included">
              <span />
            </Localized>
          </Badge>
        </li>
        <li className="help-row">
          <span className="help-row-icon" aria-hidden="true">
            <Lock />
          </span>
          <div className="help-row-copy">
            <Localized id="help-never-title">
              <h2 />
            </Localized>
            <ul className="help-list">
              {NEVER.map((id) => (
                <Localized key={id} id={id}>
                  <li />
                </Localized>
              ))}
            </ul>
          </div>
          <Badge icon={<Lock />}>
            <Localized id="help-chip-excluded">
              <span />
            </Localized>
          </Badge>
        </li>
      </ul>

      <div className="note">
        <Lock />
        <Localized id="help-note">
          <p className="note-text" />
        </Localized>
      </div>

      <div className="people-actions">
        <Button variant="primary" onClick={() => void doCopy()}>
          {l10n.getString("help-copy")}
        </Button>
        {canShare && (
          <Button
            onClick={() => {
              onShare(code, l10n.getString("help-share-text", { code }));
            }}
          >
            {l10n.getString("help-share")}
          </Button>
        )}
      </div>
      {canShare && (
        <Localized id="help-share-hint">
          <p className="people-note" />
        </Localized>
      )}
      <div className="people-summary" role="status">
        {result === "copied" && (
          <Localized id="help-copied">
            <p />
          </Localized>
        )}
        {result === "failed" && (
          <Localized id="help-copy-failed">
            <p />
          </Localized>
        )}
      </div>
    </div>
  );
}
