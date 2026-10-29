/**
 * Smaller new laptop (Task List 2.6): the plan does not fit this laptop, and PCTwin proposes
 * leaving the files used longest ago on the old laptop (never the ones that move first, only as
 * much as is short). One plain box with one choice, unticked until the person ticks it. Nothing
 * is deleted anywhere: those files stay on the old laptop. When even then something is still
 * short, the box says how much, and that some folders need unticking or another place.
 */
import { useLocalization } from "@fluent/react";
import { Banner, Checkbox } from "../components/kit";
import { PlanBox, useFormat } from "./PlanParts";

export type Proposal = { files: number; bytes: number; stillShortBytes: number };

/** Whether the plan fits once the person's answer to the proposal is counted. */
export function fitsWith(space: Proposal | undefined, accepted: boolean): boolean {
  return space !== undefined && accepted && space.stillShortBytes === 0;
}

export default function SpaceBox({ space, accepted, onChange }: { space: Proposal; accepted: boolean; onChange: (accepted: boolean) => void }) {
  const { l10n } = useLocalization();
  const { bytes } = useFormat();
  return (
    <PlanBox title={l10n.getString("space-box-title")}>
      <Checkbox
        checked={accepted}
        onChange={onChange}
        label={l10n.getString("space-box-choice", { count: space.files, size: bytes(space.bytes) })}
        hint={l10n.getString("space-box-kept")}
      />
      {space.stillShortBytes > 0 ? (
        <div className="mv-note">
          <Banner tone="attention">{l10n.getString("space-box-short", { size: bytes(space.stillShortBytes) })}</Banner>
        </div>
      ) : (
        accepted && <p className="kit-hint">{l10n.getString("space-box-fits")}</p>
      )}
    </PlanBox>
  );
}
