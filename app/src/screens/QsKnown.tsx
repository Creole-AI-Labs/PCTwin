import { useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import type { KnownConnection, KnownDevice, QsRefusal } from "../bridge";
import { Banner, Button, Dialog, TextField, Toggle } from "../components/kit";
import { Laptop } from "../icons";
import { dateText } from "./FinishParts";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsStrip, Tile } from "./QsParts";

/** The device line: what PCTwin can say about the laptop, never more ("same device", not a person). */
export const DEVICE_LINE: Record<KnownDevice, string> = {
  same: "qs-known-same-device",
  restored: "qs-known-restored",
  "cannot-tell": "qs-known-cannot-tell",
};

export type KnownActions = {
  onRename: (id: string, label: string) => void;
  onForget: (id: string) => void;
  /** Allow future sends into a folder: the system's picker chooses it. */
  onAllowFolder: (id: string) => void;
  onRevokeFolder: (id: string) => void;
  onRenewFolder: (id: string) => void;
  /** Folder for next time: the folder the last receive saved into. */
  onNextFolder: (id: string) => void;
  onClearNextFolder: (id: string) => void;
};

/** One remembered connection, with what it may do and the person's controls for each. */
function KnownCard({ c, problem, actions }: { c: KnownConnection; problem: QsRefusal | null; actions: KnownActions }) {
  const { l10n } = useLocalization();
  const titleId = useId();
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(c.label);
  const [forgetting, setForgetting] = useState(false);
  const allowed = c.folder !== null && !c.folder.ended;
  return (
    <li className="qs-card qs-known">
      <div className="qs-known-head">
        <Tile>
          <Laptop className="qs-small-icon" />
        </Tile>
        <span className="qs-known-text">
          <strong id={titleId}>
            <Name>{c.label}</Name>
          </strong>
          <small className="kit-hint">{l10n.getString("qs-known-last-used", { date: dateText(c.lastUsed) })}</small>
          <small className={c.device === "same" ? "kit-hint" : "qs-known-warn"}>{l10n.getString(DEVICE_LINE[c.device])}</small>
          <small className="kit-hint">{l10n.getString("qs-known-expires", { date: dateText(c.expiresAt) })}</small>
        </span>
      </div>

      {renaming ? (
        <form
          className="qs-known-rename"
          onSubmit={(e) => {
            e.preventDefault();
            if (name.trim() === "") return;
            actions.onRename(c.id, name.trim());
            setRenaming(false);
          }}
        >
          <TextField
            label={l10n.getString("qs-known-rename-label")}
            hint={l10n.getString("qs-remember-name-hint")}
            value={name}
            dir="auto"
            maxLength={40}
            autoFocus
            onChange={(e) => {
              setName(e.target.value);
            }}
          />
          <div className="qs-actions">
            <Button type="submit" disabled={name.trim() === ""}>
              {l10n.getString("qs-known-rename-save")}
            </Button>
            <Button
              variant="quiet"
              onClick={() => {
                setName(c.label);
                setRenaming(false);
              }}
            >
              {l10n.getString("pair-old-rename-cancel")}
            </Button>
          </div>
        </form>
      ) : null}

      <div className="qs-known-part">
        <Toggle
          checked={allowed}
          label={l10n.getString("qs-known-allow-folder")}
          hint={l10n.getString("qs-known-allow-body")}
          onChange={(on) => {
            if (on) actions.onAllowFolder(c.id);
            else actions.onRevokeFolder(c.id);
          }}
        />
        {c.folder && (
          <div className="qs-folder-line">
            <Tile>
              <Folder size={18} />
            </Tile>
            <span>
              {!c.folder.ended && (
                <>
                  {l10n.getString("qs-known-folder-on", { folder: c.folder.name })} {l10n.getString("qs-known-folder-ends", { date: dateText(c.folder.endsAt) })}
                </>
              )}
              {c.folder.ended && l10n.getString("qs-known-folder-ended", { date: dateText(c.folder.endsAt) })}
            </span>
            <Button
              onClick={() => {
                actions.onRenewFolder(c.id);
              }}
            >
              {l10n.getString("qs-known-renew")}
            </Button>
          </div>
        )}
      </div>

      <div className="qs-known-part">
        <strong className="qs-known-sub">{l10n.getString("qs-known-next-title")}</strong>
        <p className="qs-small">{c.nextFolder ? l10n.getString("qs-known-next-on", { folder: c.nextFolder.name }) : l10n.getString("qs-known-next-none")}</p>
        <div className="qs-actions">
          {c.nextFolder ? (
            <Button
              variant="quiet"
              onClick={() => {
                actions.onClearNextFolder(c.id);
              }}
            >
              {l10n.getString("qs-known-next-clear")}
            </Button>
          ) : (
            <Button
              variant="quiet"
              onClick={() => {
                actions.onNextFolder(c.id);
              }}
            >
              {l10n.getString("qs-known-next-use")}
            </Button>
          )}
        </div>
      </div>

      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}

      <div className="qs-actions">
        {!renaming && (
          <Button
            onClick={() => {
              setName(c.label);
              setRenaming(true);
            }}
          >
            {l10n.getString("qs-known-rename")}
          </Button>
        )}
        <Button
          variant="quiet"
          onClick={() => {
            setForgetting(true);
          }}
        >
          {l10n.getString("qs-known-forget")}
        </Button>
      </div>

      <Dialog
        open={forgetting}
        title={l10n.getString("qs-known-forget-title", { label: c.label })}
        initialFocus="close"
        closeLabel={l10n.getString("pair-old-rename-cancel")}
        onClose={() => {
          setForgetting(false);
        }}
        actions={
          <Button
            variant="primary"
            onClick={() => {
              setForgetting(false);
              actions.onForget(c.id);
            }}
          >
            {l10n.getString("qs-known-forget-confirm")}
          </Button>
        }
      >
        <p>{l10n.getString("qs-known-forget-body")}</p>
      </Dialog>
    </li>
  );
}

/**
 * Remembered connections: each laptop by the name the person gave it, when it was last used, what
 * PCTwin can say about it (the same laptop as before, restored or copied, or cannot tell), and when
 * it is forgotten on its own. Under each: Allow future sends into a folder (off at first; the
 * folder, its 30-day end and Renew), the folder for next time (kept apart from that permission),
 * Rename and Forget (asked once more). `problem` is the last refusal, shown on its connection's card.
 * Events: the KnownActions, onSend, onBack.
 */
export default function KnownList({
  connections,
  problem = null,
  failed = false,
  onSend,
  onBack,
  ...actions
}: {
  connections: KnownConnection[];
  problem?: { id: string; refusal: QsRefusal } | null;
  /** The list could not be read. */
  failed?: boolean;
  onSend: () => void;
  onBack: () => void;
} & KnownActions) {
  const { l10n } = useLocalization();
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-known-title" />
      <p className="subhead">{l10n.getString("qs-known-intro")}</p>
      {failed && <Banner tone="attention">{l10n.getString("qs-refusal-failed")}</Banner>}
      {!failed && connections.length === 0 && (
        <p role="status" className="qs-count">
          {l10n.getString("qs-known-empty")}
        </p>
      )}
      {connections.length > 0 && (
        <ul className="qs-known-list">
          {connections.map((c) => (
            <KnownCard key={c.id} c={c} problem={problem?.id === c.id ? problem.refusal : null} actions={actions} />
          ))}
        </ul>
      )}
      <FrameFooter note="send">
        <Button variant="primary" onClick={onSend}>
          {l10n.getString("qs-home-action")}
        </Button>
      </FrameFooter>
    </>
  );
}
