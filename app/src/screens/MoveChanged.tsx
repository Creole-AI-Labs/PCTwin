/**
 * Changes during the move (Task List 2.9 item 6; mockup X06): after the person said yes, something
 * changed (an app updated or was removed, a folder it goes to changed, a project was edited on the
 * old laptop). Only those items are asked about again; the steps they affect wait, and everything
 * else stays approved. The person says yes again or leaves them out. `generation` goes back as received.
 */
import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { moveReapprove, toMoveProblem } from "../bridge";
import { Banner, Button, Row, RowList } from "../components/kit";
import type { ChangesView } from "../deliver";
import { listText } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Name, MoreLine } from "./FinishParts";
import { problemKey } from "./RecoveryKit";
import { Heading } from "./ProgressParts";

export default function MoveChanged({ changes, onDone }: { changes: ChangesView; onDone: () => void }) {
  const { l10n } = useLocalization();
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const answer = (approve: boolean) => {
    setBusy(true);
    moveReapprove(changes.generation, approve).then(
      () => {
        setBusy(false);
        onDone();
      },
      (error: unknown) => {
        setBusy(false);
        setProblem(problemKey(toMoveProblem(error).kind));
      },
    );
  };
  return (
    <>
      <TopStrip labelId="changed-strip" />
      <Heading id="changed-title" vars={{ count: changes.lines.length + changes.more }} />
      <p className="subhead">{l10n.getString("changed-body")}</p>
      <div role="status">
        {problem && (
          <div className="mv-note">
            <Banner tone="attention">{l10n.getString(problem)}</Banner>
          </div>
        )}
      </div>
      <RowList>
        {changes.lines.map((c, i) => (
          <Row
            key={i}
            title={<Name>{c.name}</Name>}
            detail={[l10n.getString(`changed-why-${c.why}`), l10n.getString("changed-affects", { steps: listText(c.steps.map((s) => l10n.getString(`changed-step-${s}`))) })].join(". ")}
          />
        ))}
      </RowList>
      <MoreLine count={changes.more} />
      <p className="kit-hint">{l10n.getString("changed-held", { count: changes.held })}</p>
      <FrameFooter>
        <Button
          variant="quiet"
          disabled={busy}
          onClick={() => {
            answer(false);
          }}
        >
          {l10n.getString("changed-leave-out")}
        </Button>
        <Button
          variant="primary"
          disabled={busy}
          onClick={() => {
            answer(true);
          }}
        >
          {l10n.getString("changed-approve")}
        </Button>
      </FrameFooter>
    </>
  );
}
