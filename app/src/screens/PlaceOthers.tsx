import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Banner, Button, Row, RowList } from "../components/kit";
import type { PeopleToPlace, PlaceChoice } from "../bridge";
import { formatBytes, pageLocale } from "../move";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./people.css";
import { shownName } from "../names";

type Value = "choose" | "new" | "leave-out" | `existing:${string}`;

function choiceOf(value: Value): PlaceChoice | null {
  if (value === "choose") return null;
  if (value === "new") return { kind: "new" };
  if (value === "leave-out") return { kind: "leave-out" };
  return { kind: "existing", accountId: value.slice("existing:".length) };
}

/**
 * M06 on the new laptop: the other people whose things came from the old laptop, each with how
 * much is theirs (counted; none of their files is named), and where their things go here: an
 * account already on this laptop, a new account PCTwin makes for them, or left out. Nothing is
 * chosen for the person: each starts on "Choose". New accounts and the holding of their things
 * need an administrator's approval once, after the plan is approved; a new account's temporary
 * password is shown in the system's own window. One account is never given to two people.
 */
export default function PlaceOthers({ toPlace, onDone }: { toPlace: PeopleToPlace; onDone: (choices: { id: string; to: PlaceChoice }[]) => void }) {
  const { l10n } = useLocalization();
  const [values, setValues] = useState<Record<string, Value>>(() =>
    Object.fromEntries(toPlace.people.map((p) => [p.id, toPlace.canPlace ? "choose" : "leave-out"])),
  );
  const used = (accountId: string, except: string) => Object.entries(values).some(([id, v]) => id !== except && v === `existing:${accountId}`);
  const undecided = Object.values(values).filter((v) => v === "choose").length;
  return (
    <>
      <TopStrip labelId="people-eyebrow" />
      <Heading id="place-title" />
      <p className="subhead">{l10n.getString("place-body")}</p>
      <RowList>
        {toPlace.people.map((p) => (
          <Row
            key={p.id}
            title={<bdi dir="auto">{shownName(p.name)}</bdi>}
            detail={l10n.getString("place-size", { count: p.files, size: formatBytes(p.bytes, pageLocale()) })}
            end={
              toPlace.canPlace ? (
                <select
                  className="kit-select people-select"
                  aria-label={l10n.getString("place-select", { name: p.name })}
                  value={values[p.id]}
                  onChange={(e) => {
                    setValues((v) => ({ ...v, [p.id]: e.target.value as Value }));
                  }}
                >
                  <option value="choose">{l10n.getString("people-option-choose")}</option>
                  {toPlace.accounts.map((a) => (
                    <option key={a.id} value={`existing:${a.id}`} disabled={used(a.id, p.id)}>
                      {l10n.getString("people-option-existing", { account: a.name })}
                    </option>
                  ))}
                  <option value="new">{l10n.getString("people-option-new")}</option>
                  <option value="leave-out">{l10n.getString("people-option-leave-out")}</option>
                </select>
              ) : undefined
            }
          />
        ))}
      </RowList>
      <div className="mv-note">
        <Banner tone={toPlace.canPlace ? "info" : "attention"}>{l10n.getString(toPlace.canPlace ? "place-note" : "move-others-not-here")}</Banner>
      </div>
      {undecided > 0 && (
        <p className="plan-quiet" role="status">
          {l10n.getString("place-todo", { count: undecided })}
        </p>
      )}
      <FrameFooter>
        <Button
          variant="primary"
          disabled={undecided > 0}
          onClick={() => {
            const choices = toPlace.people.flatMap((p) => {
              const to = choiceOf(values[p.id]);
              return to ? [{ id: p.id, to }] : [];
            });
            onDone(choices);
          }}
        >
          {l10n.getString("pair-done-continue")}
        </Button>
      </FrameFooter>
    </>
  );
}
