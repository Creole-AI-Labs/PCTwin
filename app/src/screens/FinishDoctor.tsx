import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { doctorRun, toFinishProblem } from "../bridge";
import { Banner, Button, Row, RowList, Segmented } from "../components/kit";
import type { DoctorRole, DoctorView, FinishProblem } from "../finish";
import { Attention, Check } from "../icons";
import { Badge } from "../components/kit";
import { listText } from "../move";
import { FinishPage, Loading, Name, ProblemNotice, useHas } from "./FinishParts";

/**
 * Connection doctor: what PCTwin found on this laptop that could stop the two laptops finding each
 * other, each with what to do, the phone hotspot as the way round, and what could not be checked
 * (never blamed). Having found nothing is not a promise the connection works.
 */
export default function FinishDoctor({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const has = useHas();
  const [role, setRole] = useState<DoctorRole>("new");
  const [view, setView] = useState<DoctorView | null>(null);
  const [running, setRunning] = useState(false);
  const [problem, setProblem] = useState<FinishProblem | null>(null);
  const hotspot = view && (
    <p>{view.hotspot.suggested ? l10n.getString("finish-doctor-hotspot-first") : l10n.getString("finish-doctor-hotspot-last")}</p>
  );
  return (
    <FinishPage titleId="finish-doctor-title" onBack={onBack}>
      <p className="subhead">{l10n.getString("finish-doctor-body")}</p>
      <Segmented<DoctorRole>
        label={l10n.getString("finish-doctor-role-label")}
        value={role}
        options={[
          { value: "old", label: l10n.getString("finish-doctor-role-old") },
          { value: "new", label: l10n.getString("finish-doctor-role-new") },
        ]}
        onChange={setRole}
      />
      <div className="fin-actions">
        <Button
          variant="primary"
          disabled={running}
          onClick={() => {
            setRunning(true);
            setProblem(null);
            doctorRun(role).then(
              (v) => {
                setRunning(false);
                setView(v);
              },
              (error: unknown) => {
                setRunning(false);
                setProblem(toFinishProblem(error));
              },
            );
          }}
        >
          {l10n.getString("finish-doctor-run")}
        </Button>
      </div>
      {running && <Loading />}
      {problem && <ProblemNotice problem={problem} />}
      <div role="status">
        {view && !running && (
          <>
            <h2 className="fin-block">{l10n.getString("finish-doctor-found")}</h2>
            {view.findings.length === 0 && <p>{l10n.getString("finish-doctor-none")}</p>}
            <RowList>
              {view.findings.map((f, i) => {
                const vars = { names: listText(f.names) };
                return (
                  <Row
                    key={`${f.cause}-${String(i)}`}
                    icon={f.sure ? <Check className="mv-icon" /> : <Attention className="mv-icon" />}
                    title={has(f.cause) ? l10n.getString(f.cause, vars) : <Name>{f.cause}</Name>}
                    detail={`${l10n.getString("finish-doctor-fix")}: ${has(f.fix) ? l10n.getString(f.fix, vars) : f.fix}`}
                    end={<Badge tone={f.sure ? "good" : "warn"}>{l10n.getString(f.sure ? "finish-doctor-sure" : "finish-doctor-likely")}</Badge>}
                  />
                );
              })}
            </RowList>
            <Banner tone="info">{hotspot}</Banner>
            {view.notChecked.length > 0 && (
              <>
                <h2 className="fin-block">{l10n.getString("finish-doctor-not-checked")}</h2>
                <ul className="fin-list">
                  {view.notChecked.map((k) => (
                    <li key={k}>{has(`finish-notchecked-${k}`) ? l10n.getString(`finish-notchecked-${k}`) : l10n.getString("finish-notchecked-unknown")}</li>
                  ))}
                </ul>
              </>
            )}
          </>
        )}
      </div>
    </FinishPage>
  );
}
