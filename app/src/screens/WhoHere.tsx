import { useLocalization } from "@fluent/react";
import { useState } from "react";
import { Banner, Button, Checkbox, Row, RowList } from "../components/kit";
import type { PeopleHere } from "../bridge";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./progress.css";
import { shownName } from "../names";

/**
 * M06 Who's moving?, on the old laptop: everyone with an account here. The mover is always
 * moving; ticking anyone else asks, once, for an administrator's approval (M25), and then all of
 * them move in one go. Their files are never listed to the mover, only their names. On a system
 * that cannot move other people yet, they are listed but cannot be ticked, and the page says so.
 */
export default function WhoHere({ here, onContinue }: { here: PeopleHere; onContinue: (others: string[]) => void }) {
  const { l10n } = useLocalization();
  const [ticked, setTicked] = useState<string[]>([]);
  const others = here.people.filter((p) => !p.isYou);
  return (
    <>
      <TopStrip labelId="move-wait-eyebrow" />
      <Heading id="who-here-title" />
      <p className="subhead">{l10n.getString("who-here-body")}</p>
      <RowList>
        {here.people
          .filter((p) => p.isYou)
          .map((p) => (
            <Row key={p.id} title={<bdi dir="auto">{l10n.getString("who-here-you", { name: shownName(p.name) })}</bdi>} detail={l10n.getString("who-here-you-hint")} />
          ))}
        {!here.canMoveOthers && others.map((p) => <Row key={p.id} title={<bdi dir="auto">{shownName(p.name)}</bdi>} />)}
      </RowList>
      {here.canMoveOthers && others.length > 0 && (
        <ul className="mv-people">
          {others.map((p) => (
            <li key={p.id}>
              <Checkbox
                checked={ticked.includes(p.id)}
                onChange={(on) => {
                  setTicked((t) => (on ? [...t, p.id] : t.filter((id) => id !== p.id)));
                }}
                label={<bdi dir="auto">{shownName(p.name)}</bdi>}
              />
            </li>
          ))}
        </ul>
      )}
      {others.length > 0 && (
        <div className="mv-note">
          <Banner tone={here.canMoveOthers ? "info" : "attention"}>{l10n.getString(here.canMoveOthers ? "who-here-approval" : "who-here-not-here")}</Banner>
        </div>
      )}
      <FrameFooter>
        <Button
          variant="primary"
          onClick={() => {
            onContinue(ticked);
          }}
        >
          {l10n.getString("pair-done-continue")}
        </Button>
      </FrameFooter>
    </>
  );
}
