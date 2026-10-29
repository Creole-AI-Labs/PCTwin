/**
 * Rescue a laptop (Task List 2.6; blueprint T01 to T03), from Home: bring files back from a laptop
 * that will not start, by connecting its drive to this one. The engine lists the drives connected
 * here (never the one this laptop runs from); the person chooses one, and the new laptop's move
 * follows, with the drive as the old laptop. The drive is only read.
 *
 * - No drive: how to connect one, and Check again.
 * - The list could not be read: said so, with Check again (never shown as "no drive").
 * - A locked drive: the system asks for its key in its own window; where the key can be found;
 *   PCTwin never asks for the key and never keeps it.
 */
import { useLocalization } from "@fluent/react";
import { useCallback, useEffect, useState } from "react";
import { Badge, Banner, Button } from "../components/kit";
import { rescueDrives, type RescueDrive } from "../bridge";
import { Shield } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Name } from "./GuardParts";
import { DriveIcon, useFormat } from "./PlanParts";
import { Heading } from "./ProgressParts";

type Listed = { kind: "checking" } | { kind: "failed" } | { kind: "ready"; drives: RescueDrive[] };

export default function RescuePick({
  onBack,
  onChoose,
  load = rescueDrives,
}: {
  onBack: () => void;
  onChoose: (drive: RescueDrive) => void;
  /** Where the drives come from (the engine; a stand-in in tests). */
  load?: () => Promise<RescueDrive[]>;
}) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  const [listed, setListed] = useState<Listed>({ kind: "checking" });
  // Asks the engine; the answer (or its failure) replaces what is shown.
  const ask = useCallback(() => {
    load().then(
      (drives) => {
        setListed({ kind: "ready", drives });
      },
      () => {
        setListed({ kind: "failed" });
      },
    );
  }, [load]);
  // The page opens "checking" and asks once.
  useEffect(() => {
    ask();
  }, [ask]);
  const check = () => {
    setListed({ kind: "checking" });
    ask();
  };

  return (
    <>
      <TopStrip onBack={onBack} labelId="rescue-eyebrow" />
      <Heading id="rescue-title" />
      <p className="subhead">{l10n.getString("rescue-lead")}</p>
      <p className="rc-safe">
        <Shield />
        <span>{l10n.getString("guard-rescue-read-only-promise")}</span>
      </p>
      <div role="status" className="rc-status">
        {listed.kind === "checking" && <p>{l10n.getString("rescue-checking")}</p>}
        {listed.kind === "failed" && <Banner tone="attention">{l10n.getString("rescue-failed")}</Banner>}
        {listed.kind === "ready" && listed.drives.length === 0 && <p className="mv-empty">{l10n.getString("rescue-empty")}</p>}
      </div>
      {listed.kind === "ready" && listed.drives.length > 0 && (
        <ul className="kit-rows rc-drives">
          {listed.drives.map((d) => (
            <li key={d.id} className="kit-row">
              <span className="kit-row-name">
                <span className="kit-row-icon">
                  <DriveIcon />
                </span>
                <span>
                  <Name>{d.name}</Name>
                  <small className="kit-hint">{l10n.getString("guard-rescue-drive-detail", { size: bytes(d.totalBytes), fs: d.fileSystem })}</small>
                  {d.looksLikeALaptop && <Badge>{l10n.getString("rescue-looks-like-laptop")}</Badge>}
                </span>
              </span>
              <span className="kit-row-end">
                <Button
                  onClick={() => {
                    onChoose(d);
                  }}
                >
                  {l10n.getString("rescue-choose", { name: d.name })}
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}
      <details className="mv-details rc-locked">
        <summary>{l10n.getString("rescue-locked-title")}</summary>
        <p>{l10n.getString("rescue-locked-windows")}</p>
        <p>{l10n.getString("guard-rescue-bit-locker-step-2")}</p>
        <p>{l10n.getString("guard-rescue-file-vault-step-1")}</p>
        <p>{l10n.getString("guard-rescue-never-key")}</p>
      </details>
      <FrameFooter>
        <Button variant={listed.kind === "ready" && listed.drives.length > 0 ? "secondary" : "primary"} onClick={check}>
          {l10n.getString("rescue-check-again")}
        </Button>
      </FrameFooter>
    </>
  );
}
