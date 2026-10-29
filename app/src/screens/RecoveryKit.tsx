/**
 * The offline recovery kit (Task List 2.9 item 7; Engineering Plan A02): a small record of one move,
 * saved in a folder the person chooses. It says before and after saving what the kit holds and what
 * it does not: it is not a copy of the files and cannot bring back missing file data. Nothing is
 * sent anywhere. A folder that was not chosen saves nothing, and the page says so.
 */
import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { historyList, recoveryKitSave, toMoveProblem } from "../bridge";
import { Banner, Button, Row, RowList, Section } from "../components/kit";
import type { KitSaved } from "../deliver";
import { Check, Info } from "../icons";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Name } from "./FinishParts";
import { Heading } from "./ProgressParts";

/** The shared words for a refused command (the engine's move problems). */
export function problemKey(kind: ReturnType<typeof toMoveProblem>["kind"]): string {
  if (kind === "busy") return "finish-problem-busy";
  if (kind === "plan-changed" || kind === "not-moving") return "finish-problem-changed";
  return "finish-problem-failed";
}

type Result = { kind: "saved"; kit: KitSaved } | { kind: "none" } | { kind: "problem"; key: string };

/** `id` is the move's My PCTwin history ID; none means the newest move. */
export default function RecoveryKit({ id, onBack }: { id: string | null; onBack: () => void }) {
  const { l10n } = useLocalization();
  const [result, setResult] = useState<Result | null>(null);
  const [busy, setBusy] = useState(false);

  const save = async () => {
    setBusy(true);
    try {
      const target = id ?? (await historyList()).sort((a, b) => Date.parse(b.when) - Date.parse(a.when))[0]?.id;
      if (!target) {
        setResult({ kind: "problem", key: "finish-problem-no-move" });
        return;
      }
      const kit = await recoveryKitSave(target);
      setResult(kit === null ? { kind: "none" } : { kind: "saved", kit });
    } catch (error: unknown) {
      setResult({ kind: "problem", key: problemKey(toMoveProblem(error).kind) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <TopStrip onBack={onBack} labelId="kit-strip" />
      <Heading id="kit-title" />
      <p className="subhead">{l10n.getString("kit-body")}</p>
      <div className="mv-note">
        <Banner tone="info">{l10n.getString("kit-lacks-sentence")}</Banner>
      </div>
      <div role="status">
        {result?.kind === "none" && (
          <div className="mv-note">
            <Banner tone="info">{l10n.getString("kit-not-saved")}</Banner>
          </div>
        )}
        {result?.kind === "problem" && (
          <div className="mv-note">
            <Banner tone="attention">{l10n.getString(result.key)}</Banner>
          </div>
        )}
        {result?.kind === "saved" && (
          <div className="mv-note">
            <Banner tone="ready">
              {l10n.getString("kit-saved-to")} <Name>{result.kit.folder}</Name>
            </Banner>
          </div>
        )}
      </div>
      {result?.kind === "saved" && (
        <>
          <Section title={l10n.getString("kit-files-title")}>
            <RowList>
              {result.kit.files.map((f) => (
                <Row key={f} title={<Name>{f}</Name>} />
              ))}
            </RowList>
          </Section>
          <Section title={l10n.getString("kit-holds-title")}>
            <RowList>
              {result.kit.holds.map((p) => (
                <Row key={p} icon={<Check className="mv-icon" />} title={l10n.getString(`kit-part-${p}`)} />
              ))}
            </RowList>
          </Section>
          {result.kit.lacks.length > 0 && (
            <Section title={l10n.getString("kit-lacks-title")}>
              <RowList>
                {result.kit.lacks.map((p) => (
                  <Row key={p} icon={<Info className="mv-icon" />} title={l10n.getString(`kit-part-${p}`)} />
                ))}
              </RowList>
            </Section>
          )}
        </>
      )}
      <FrameFooter>
        <Button
          variant="primary"
          disabled={busy}
          onClick={() => {
            void save();
          }}
        >
          {l10n.getString(result?.kind === "saved" ? "kit-save-again" : "kit-save")}
        </Button>
      </FrameFooter>
    </>
  );
}
