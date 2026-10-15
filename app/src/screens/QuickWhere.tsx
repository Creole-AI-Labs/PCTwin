import { Localized, useLocalization } from "@fluent/react";
import { Badge, Banner, Button, Checkbox, Row } from "../components/kit";
import { Attention, Check, Info } from "../icons";
import { useBytes } from "../quicksend";
import FrameFooter from "./FrameFooter";
import { Folder, Heading, Name, QsActions, QsRowList, QsStrip, Tile } from "./QsParts";

export type WhereProblem = "unavailable" | "no-permission";

/**
 * Q09 Where should the files land? (receiver) The receiving folder, the space check, and the box that
 * says this folder is for this offer. The person picks the folder through the system picker; the sender
 * has no say and every file stays inside it. Space is measured: `free` is null when it could not be
 * measured, and then the screen says so rather than guessing. A folder that is not there, one PCTwin may
 * not write to, or one without room blocks the main action, as does an unticked box.
 * Events: onChoose (open the folder picker), onConfirmedChange, onContinue, onBack.
 */
export default function QuickWhere({
  folder,
  free,
  needed,
  problem,
  confirmed,
  onConfirmedChange,
  onChoose,
  onContinue,
  onBack,
}: {
  folder: string | null;
  free: number | null;
  needed: number;
  problem?: WhereProblem;
  confirmed: boolean;
  onConfirmedChange: (on: boolean) => void;
  onChoose: () => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const bytes = useBytes();
  const tooSmall = folder !== null && free !== null && free < needed;
  const blocked = folder === null || problem !== undefined || tooSmall || !confirmed;
  return (
    <>
      <QsStrip onBack={onBack} receiving />
      <Heading id="qs-land-title" />
      <Localized id="qs-land-body">
        <p className="subhead" />
      </Localized>

      <div className="qs-field">
        <Localized id="qs-land-folder-label">
          <span id="qs-land-folder" className="qs-field-label" />
        </Localized>
        <div className="qs-field-box" aria-labelledby="qs-land-folder" role="group">
          {folder ? (
            <>
              <Folder size={18} />
              <Name>{folder}</Name>
            </>
          ) : (
            <Localized id="qs-land-none">
              <span />
            </Localized>
          )}
        </div>
      </div>
      <QsActions>
        <button type="button" className="link" onClick={onChoose}>
          <Localized id="qs-land-choose">
            <span />
          </Localized>
        </button>
      </QsActions>

      <div role="status" className="qs-status">
        {problem && (
          <Banner tone="attention">
            <Localized id={problem === "unavailable" ? "qs-land-unavailable" : "qs-land-denied"}>
              <p />
            </Localized>
          </Banner>
        )}
        {tooSmall && (
          <Banner tone="attention">
            <Localized id="qs-land-no-space" vars={{ free: bytes(free), needed: bytes(needed) }}>
              <p />
            </Localized>
          </Banner>
        )}
      </div>

      {folder && !problem && (
        <QsRowList>
          <Row
            icon={
              <Tile>
                <Folder size={18} />
              </Tile>
            }
            title={
              <Localized id="qs-land-space-title">
                <span />
              </Localized>
            }
            detail={
              free === null
                ? l10n.getString("qs-land-space-unknown", { needed: bytes(needed) })
                : l10n.getString("qs-land-space", { free: bytes(free), needed: bytes(needed) })
            }
            end={
              free === null ? undefined : tooSmall ? (
                <Badge tone="warn" icon={<Attention />}>
                  {l10n.getString("qs-land-status-no-space")}
                </Badge>
              ) : (
                <Badge tone="good" icon={<Check />}>
                  {l10n.getString("qs-land-status-fits")}
                </Badge>
              )
            }
          />
        </QsRowList>
      )}

      <div className="qs-block qs-card">
        <Checkbox
          checked={confirmed}
          onChange={onConfirmedChange}
          label={
            <Localized id="qs-land-confirm">
              <span />
            </Localized>
          }
          hint={
            <Localized id="qs-land-confirm-hint">
              <span />
            </Localized>
          }
        />
      </div>

      <div className="note">
        <Info />
        <Localized id="qs-land-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter note="send">
        <Button variant="primary" disabled={blocked} onClick={onContinue}>
          <Localized id="qs-land-use">
            <span />
          </Localized>
        </Button>
      </FrameFooter>
    </>
  );
}
