import { Localized, useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import { qsOpenLink, toQsRefusal, type QsNote, type QsRefusal } from "../bridge";
import { Banner, Button } from "../components/kit";
import FrameFooter from "./FrameFooter";
import { Heading, QsActions, QsStrip } from "./QsParts";

/** The same limits the engine checks (`pctwin_quicksend::note`), said before anything is sent. */
export const NOTE_CHARS = 1000;
export const NOTE_LINES = 20;

/** Why a typed note cannot go, by the engine's own rules, or null when it can. */
export function noteProblem(text: string): QsRefusal | null {
  const tidy = text.replace(/\r\n/g, "\n").trim();
  if (tidy === "") return { kind: "note-empty" };
  if (Array.from(tidy).length > NOTE_CHARS) return { kind: "note-too-long" };
  if (tidy.split("\n").length > NOTE_LINES) return { kind: "note-too-many-lines" };
  return null;
}

/**
 * Q24 Send a note or link: a few typed lines or one web link, typed or pasted on purpose (nothing
 * reads the clipboard). Continue pairs with the code, and the note is sent the moment the laptops
 * are paired. `problem` is the reason the last try was refused, said under the box.
 * Events: onChange(text), onContinue, onBack.
 */
export function NoteCompose({
  text,
  problem,
  onChange,
  onContinue,
  onBack,
}: {
  text: string;
  problem: QsRefusal | null;
  onChange: (text: string) => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const id = useId();
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-note-title" />
      <Localized id="qs-note-intro">
        <p className="subhead" />
      </Localized>
      <div className="kit-field qs-note-field">
        <label htmlFor={id}>{l10n.getString("qs-note-label")}</label>
        <textarea
          id={id}
          className="qs-note-box"
          dir="auto"
          rows={6}
          value={text}
          spellCheck
          aria-describedby={`${id}-hint`}
          aria-invalid={problem ? true : undefined}
          onChange={(e) => {
            onChange(e.target.value);
          }}
        />
        <small id={`${id}-hint`} className="kit-hint">
          {l10n.getString("qs-note-hint")}
        </small>
      </div>
      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}
      <Banner tone="info">{l10n.getString("qs-note-never-opens")}</Banner>
      <FrameFooter note="send">
        <Button variant="primary" disabled={text.trim() === ""} onClick={onContinue}>
          {l10n.getString("qs-choose-continue")}
        </Button>
      </FrameFooter>
    </>
  );
}

/** The sender, after pairing: the note is on its way (`sent` false) or the other laptop took it. */
export function NoteSent({ sent, onDone }: { sent: boolean; onDone: () => void }) {
  const { l10n } = useLocalization();
  return (
    <>
      <QsStrip />
      <Heading id={sent ? "qs-note-sent-title" : "qs-note-sending-title"} />
      <p className="subhead" role="status">
        {l10n.getString(sent ? "qs-note-sent-body" : "qs-note-sending-body")}
      </p>
      {sent && (
        <FrameFooter note="send">
          <Button variant="primary" onClick={onDone}>
            {l10n.getString("qs-done-done")}
          </Button>
        </FrameFooter>
      )}
    </>
  );
}

/**
 * A note or link that arrived, shown as plain text exactly as sent, with Copy. A link names the
 * site it goes to in plain words and opens only by its own Open (which names this note's id, so a
 * later note never takes its place); nothing opens by itself. `from`: the remembered connection's
 * name, or null for the laptop paired with the code. `copy` / `openFailed` say how the last press went.
 * Events: onCopy, onOpen, onDone.
 */
export function NoteReceived({
  note,
  copy,
  open,
  onCopy,
  onOpen,
  onDone,
}: {
  note: QsNote;
  copy?: "copied" | "failed" | null;
  open?: QsRefusal | null;
  onCopy: () => void;
  onOpen: () => void;
  onDone: () => void;
}) {
  const { l10n } = useLocalization();
  const link = note.kind === "link";
  return (
    <>
      <QsStrip receiving />
      <Heading id={link ? "qs-note-received-link-title" : "qs-note-received-title"} />
      <p className="subhead">
        {note.from ? (
          <Localized id="qs-note-from" vars={{ label: note.from }}>
            <span />
          </Localized>
        ) : (
          l10n.getString("qs-note-from-code")
        )}
      </p>
      <div className="qs-note-shown" dir="auto" tabIndex={0} aria-label={l10n.getString("qs-note-label")}>
        {note.text}
      </div>
      {link && note.host && (
        <p className="qs-note-host">
          <Localized id="qs-note-link-to" vars={{ host: note.host }}>
            <strong />
          </Localized>{" "}
          {l10n.getString("qs-note-link-safety")}
        </p>
      )}
      <QsActions>
        <Button onClick={onCopy}>{l10n.getString("qs-note-copy")}</Button>
        {link && <Button onClick={onOpen}>{l10n.getString("qs-note-open")}</Button>}
      </QsActions>
      <div role="status" className="qs-status">
        {copy === "copied" && <p className="qs-small">{l10n.getString("qs-note-copied")}</p>}
      </div>
      {copy === "failed" && <Banner tone="attention">{l10n.getString("qs-note-copy-failed")}</Banner>}
      {open && <Banner tone="attention">{l10n.getString(`qs-refusal-${open.kind}`)}</Banner>}
      <FrameFooter note="send">
        <Button variant="primary" onClick={onDone}>
          {l10n.getString("qs-done-done")}
        </Button>
      </FrameFooter>
    </>
  );
}

/**
 * A received note with the engine: Copy puts the text on the clipboard (only because the person
 * chose it), Open asks the engine to open this note's link by its id.
 */
export function NoteInHost({ note, onDone }: { note: QsNote; onDone: () => void }) {
  const [copy, setCopy] = useState<"copied" | "failed" | null>(null);
  const [open, setOpen] = useState<QsRefusal | null>(null);
  return (
    <NoteReceived
      note={note}
      copy={copy}
      open={open}
      onCopy={() => {
        setCopy(null);
        Promise.resolve()
          .then(() => navigator.clipboard.writeText(note.text))
          .then(
            () => {
              setCopy("copied");
            },
            () => {
              setCopy("failed");
            },
          );
      }}
      onOpen={() => {
        setOpen(null);
        qsOpenLink(note.id).catch((error: unknown) => {
          setOpen(toQsRefusal(error));
        });
      }}
      onDone={onDone}
    />
  );
}
