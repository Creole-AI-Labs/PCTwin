import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { cleanupConfirm, cleanupPreview, toFinishProblem } from "../bridge";
import { Banner, Button, Checkbox, Row, RowList, Section } from "../components/kit";
import { CLEANUP_WHATS, type CleanupDone, type CleanupEntry, type CleanupWhat, type FinishProblem } from "../finish";
import { formatBytes, pageLocale } from "../move";
import { FinishPage, Loaded, Name, ProblemNotice, useLoad } from "./FinishParts";

const SHORT: Record<CleanupWhat, "temp" | "model"> = { "temporary-files": "temp", "model-and-cache": "model" };

/**
 * R07 Clean up: each question is its own page (temporary files; PCTwin's model and saved data),
 * shows what would be removed and what is kept, and removes nothing until the person ticks and confirms.
 */
export default function FinishCleanup({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const [what, setWhat] = useState<CleanupWhat | null>(null);
  if (what) {
    return (
      <Question
        what={what}
        onBack={() => {
          setWhat(null);
        }}
      />
    );
  }
  return (
    <FinishPage titleId="finish-cleanup-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-cleanup-body")}</p>
      <RowList>
        {CLEANUP_WHATS.map((w) => {
          const title = l10n.getString(`finish-cleanup-${SHORT[w]}`);
          return (
            <Row
              key={w}
              title={title}
              detail={l10n.getString(`finish-cleanup-${SHORT[w]}-detail`)}
              end={
                <Button
                  variant="quiet"
                  aria-label={l10n.getString("finish-open-name", { title })}
                  onClick={() => {
                    setWhat(w);
                  }}
                >
                  {l10n.getString("finish-cleanup-look")}
                </Button>
              }
            />
          );
        })}
      </RowList>
    </FinishPage>
  );
}

function Entries({ list }: { list: CleanupEntry[] }) {
  return (
    <RowList>
      {list.map((e, i) => (
        <Row key={`${e.name}-${String(i)}`} title={<Name>{e.name}</Name>} detail={formatBytes(e.bytes, pageLocale())} />
      ))}
    </RowList>
  );
}

function Question({ what, onBack }: { what: CleanupWhat; onBack: () => void }) {
  const { l10n } = useLocalization();
  const { state, reload } = useLoad(() => cleanupPreview(what));
  const [ticked, setTicked] = useState(false);
  const [done, setDone] = useState<CleanupDone | null>(null);
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  return (
    <FinishPage titleId={`finish-cleanup-${SHORT[what]}`} onBack={onBack}>
      {problem && <ProblemNotice problem={problem} onRetry={problem.kind === "changed" ? () => { setProblem(null); setTicked(false); reload(); } : undefined} />}
      <div role="status">{done && <Banner tone="ready">{l10n.getString("finish-cleanup-done", { removed: done.removed.toLocaleString(), kept: done.kept.toLocaleString(), failed: done.failed.toLocaleString() })}</Banner>}</div>
      <Loaded state={state} onRetry={reload}>
        {(view) => (
          <>
            <Section title={l10n.getString("finish-cleanup-removes")}>
              {view.removes.length === 0 ? <p>{l10n.getString("finish-cleanup-nothing")}</p> : <Entries list={view.removes} />}
            </Section>
            {view.keeps.length > 0 && (
              <Section title={l10n.getString("finish-cleanup-keeps")}>
                <p className="subhead">{l10n.getString("finish-cleanup-keeps-body")}</p>
                <Entries list={view.keeps} />
              </Section>
            )}
            {view.removes.length > 0 && !done && (
              <>
                <div className="mv-confirm">
                  <Checkbox checked={ticked} onChange={setTicked} label={l10n.getString("finish-cleanup-tick")} />
                </div>
                <div className="fin-actions">
                  <Button
                    variant="primary"
                    disabled={!ticked}
                    onClick={() => {
                      cleanupConfirm(what, view.digest).then(
                        (r) => {
                          setProblem(null);
                          setDone(r);
                        },
                        (error: unknown) => {
                          setTicked(false);
                          setProblem(toFinishProblem(error));
                        },
                      );
                    }}
                  >
                    {l10n.getString("finish-cleanup-confirm")}
                  </Button>
                </div>
              </>
            )}
          </>
        )}
      </Loaded>
    </FinishPage>
  );
}
