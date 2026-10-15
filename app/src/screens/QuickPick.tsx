import { Localized, useLocalization } from "@fluent/react";
import { Banner, Button, Checkbox } from "../components/kit";
import { totalBytes, useBytes, type PickedItem } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Heading, Name, QsStrip, TallyBar, useAttribute } from "./QsParts";

const PROBLEM_TEXT = { "online-only": "qs-online-only", unreadable: "qs-protected", missing: "qs-gone" } as const;

/**
 * Q02 What are you sending? The tally, the chosen items as a ticked list (untick to leave one out), and
 * Add files / Add folders under it, with a drop area, so the whole screen works by keyboard. The screen
 * never reads a path: the buttons ask the bridge to open the system picker, and the shell hands dropped
 * paths to the bridge through the window's drag-drop event (`dragging` says one is over the window).
 * A ticked item the engine cannot use is shown with the reason and blocks the main action until it is
 * unticked or made readable (Allow access asks the bridge to request permission).
 * Events: onAddFiles, onAddFolders, onToggle(id, on), onAllowAccess(id), onContinue, onBack.
 */
export default function QuickPick({
  items,
  dragging = false,
  onAddFiles,
  onAddFolders,
  onToggle,
  onAllowAccess,
  onContinue,
  onBack,
}: {
  items: PickedItem[];
  dragging?: boolean;
  onAddFiles: () => void;
  onAddFolders: () => void;
  onToggle: (id: string, on: boolean) => void;
  onAllowAccess: (id: string) => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const attribute = useAttribute();
  const bytes = useBytes();
  const chosen = items.filter((i) => i.included);
  const usable = chosen.filter((i) => !i.problem);
  const problems = chosen.filter((i) => i.problem).length;
  const sizeKnown = usable.every((i) => i.bytes !== null);
  const blocked = chosen.length === 0 || problems > 0;

  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-choose-title" />
      <Localized id="qs-choose-body">
        <p className="subhead" />
      </Localized>

      {chosen.length === 0 ? (
        <p role="status" className="qs-count">
          {l10n.getString("qs-empty")}
        </p>
      ) : sizeKnown ? (
        <TallyBar live count={chosen.length} size={bytes(totalBytes(usable.map((i) => ({ size: i.bytes ?? 0 }))))} />
      ) : (
        <p role="status" className="qs-count">
          {l10n.getString("qs-tally-counting", { count: chosen.length })}
        </p>
      )}

      {items.length > 0 && (
        <ul className="qs-offer">
          {items.map((item) => (
            <li key={item.id}>
              <Checkbox
                checked={item.included}
                onChange={(on) => {
                  onToggle(item.id, on);
                }}
                label={<Name>{item.name}</Name>}
                hint={
                  item.problem
                    ? l10n.getString(PROBLEM_TEXT[item.problem], { name: item.name })
                    : item.description && item.bytes !== null
                      ? l10n.getString("qs-file-detail", { size: bytes(item.bytes), kind: item.description })
                      : item.kind === "folder"
                      ? item.files !== null && item.bytes !== null
                        ? l10n.getString("qs-tally", { count: item.files, size: bytes(item.bytes) })
                        : null
                      : item.bytes !== null
                        ? bytes(item.bytes)
                        : null
                }
              />
              {item.problem === "unreadable" && item.included && (
                <div className="qs-row-action">
                  <Button
                    variant="quiet"
                    onClick={() => {
                      onAllowAccess(item.id);
                    }}
                  >
                    {l10n.getString("qs-allow")}
                  </Button>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}

      {problems > 0 && (
        <div role="status" className="qs-status">
          <Banner tone="attention">
            <Localized id="qs-choose-why-problems">
              <p />
            </Localized>
          </Banner>
        </div>
      )}

      <section className={`qs-drop${dragging ? " qs-drop-over" : ""}`} aria-label={attribute("qs-drop", "aria-label")}>
        <Localized id="qs-drop-title">
          <h2 className="qs-drop-title" />
        </Localized>
        <Localized id={dragging ? "qs-drop-active" : "qs-drop-body"}>
          <p className="qs-small" />
        </Localized>
        <div className="qs-links">
          <button type="button" className="link" onClick={onAddFiles}>
            <Localized id="qs-add-files">
              <span />
            </Localized>
          </button>
          <button type="button" className="link" onClick={onAddFolders}>
            <Localized id="qs-add-folders">
              <span />
            </Localized>
          </button>
        </div>
      </section>

      <FrameFooter note="send">
        <Button variant="primary" disabled={blocked} onClick={onContinue}>
          <Localized id="qs-choose-continue">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
