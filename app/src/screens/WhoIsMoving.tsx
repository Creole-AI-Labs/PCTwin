import { Localized, useLocalization } from "@fluent/react";
import type { ReactNode } from "react";
import { Badge, Button } from "../components/kit";
import { ArrowForward, Attention, Check, Info } from "../icons";
import type { Account, Destination, Person, SharedHome } from "../people";
import FrameFooter from "./FrameFooter";
import { TopStrip } from "./FrameParts";
import { Heading } from "./PairParts";
import "./people.css";
import { shownName } from "../names";

/** What a select shows for a person: a choice, or "not chosen" (also when the account chosen no longer exists). */
function valueOf(destination: Destination, accounts: readonly Account[]): string {
  switch (destination.kind) {
    case "existing":
      return accounts.some((a) => a.id === destination.accountId) ? `existing:${destination.accountId}` : "undecided";
    case "new":
      return "new";
    case "leave-out":
      return "leave-out";
    case "undecided":
      return "undecided";
  }
}

function destinationOf(value: string): Destination {
  if (value.startsWith("existing:")) return { kind: "existing", accountId: value.slice("existing:".length) };
  if (value === "new") return { kind: "new" };
  if (value === "leave-out") return { kind: "leave-out" };
  return { kind: "undecided" };
}

type Props = {
  /** The profiles on the old laptop. Your own is shown first whatever the order. */
  people: readonly Person[];
  /** The accounts already on the new laptop. */
  accounts: readonly Account[];
  shared: SharedHome;
  onBack: () => void;
  onChoose: (personId: string, destination: Destination) => void;
  onChooseShared: (shared: SharedHome) => void;
  onContinue: () => void;
};

/**
 * M06 Who is moving? The mockup's table: each profile on the old laptop, the account its files go to
 * on the new one, and its status. A name that matches an account is only a suggestion: the person
 * chooses, and nothing is assumed. Another person's profile can be mapped only after its owner has
 * signed in on the old laptop and approved; until then it waits, in words, and does not hold up the
 * rest. Shared items are the last row of the same table.
 */
export default function WhoIsMoving({ people, accounts, shared, onBack, onChoose, onChooseShared, onContinue }: Props) {
  const { l10n } = useLocalization();
  const ordered = [...people].sort((a, b) => Number(b.isYou) - Number(a.isYou));
  const todo = ordered.filter((p) => p.state === "ready" && valueOf(p.destination, accounts) === "undecided").length;
  const waiting = ordered.filter((p) => p.state === "waiting-for-owner").length;
  const changed = ordered.some((p) => p.changedByYou);
  const justMe = ordered.length === 1 && ordered[0].isYou;
  const you = ordered.find((p) => p.isYou);

  return (
    <div className="people">
      <TopStrip onBack={onBack} labelId="people-eyebrow" />
      <Heading id="people-title" />
      <Localized id="people-body">
        <p className="subhead" />
      </Localized>

      <div className="people-table-wrap">
        <table className="people-table" aria-label={l10n.getString("people-list-label")}>
          <thead>
            <tr>
              <Localized id="people-col-old">
                <th scope="col" />
              </Localized>
              <Localized id="people-col-new">
                <th scope="col" />
              </Localized>
              <Localized id="people-col-status">
                <th scope="col" />
              </Localized>
            </tr>
          </thead>
          <tbody>
            {ordered.map((person) => (
              <PersonRow key={person.id} person={person} accounts={accounts} onChoose={onChoose} />
            ))}
            <tr className="people-shared">
              <td>
                <Label id="people-col-old" />
                <Localized id="people-shared-title">
                  <strong className="people-name" />
                </Localized>
                <Localized id="people-shared-body">
                  <small className="kit-hint" />
                </Localized>
              </td>
              <td>
                <Label id="people-col-new" />
                <Localized id="people-shared-select" attrs={{ "aria-label": true }}>
                  <select
                    className="kit-select people-select"
                    value={shared}
                    onChange={(e) => {
                      onChooseShared(e.target.value as SharedHome);
                    }}
                  >
                    <option value="shared">{l10n.getString("people-shared-option-shared")}</option>
                    {you && <option value="person">{l10n.getString("people-shared-option-person", { name: you.name })}</option>}
                    <option value="leave-out">{l10n.getString("people-shared-option-leave-out")}</option>
                  </select>
                </Localized>
              </td>
              <td />
            </tr>
          </tbody>
        </table>
      </div>
      {justMe && (
        <Localized id="people-just-me">
          <p className="people-note" />
        </Localized>
      )}

      <div className="people-summary" role="status">
        <Localized id="people-todo" vars={{ count: todo }}>
          <p />
        </Localized>
        {waiting > 0 && (
          <Localized id="people-waiting-count" vars={{ count: waiting }}>
            <p />
          </Localized>
        )}
        {changed && (
          <Localized id="people-changed-note">
            <p />
          </Localized>
        )}
      </div>

      <div className="note note-warn">
        <Attention />
        <Localized id="people-note">
          <p className="note-text" />
        </Localized>
      </div>

      <FrameFooter>
        <Button variant="primary" disabled={todo > 0} icon={<ArrowForward className="flip" />} onClick={onContinue}>
          {l10n.getString("people-continue")}
        </Button>
      </FrameFooter>
    </div>
  );
}

/** One status, as an icon and a word. */
function Status({ id, tone, icon }: { id: string; tone: "good" | "warn" | "neutral"; icon: ReactNode }) {
  return (
    <Badge tone={tone} icon={icon}>
      <Localized id={id}>
        <span />
      </Localized>
    </Badge>
  );
}

/** The column's name, shown only when the table is stacked into one column (a narrow window or large text). */
function Label({ id }: { id: string }) {
  return (
    <Localized id={id}>
      <span className="people-cell-label" aria-hidden="true" />
    </Localized>
  );
}

function PersonRow({ person, accounts, onChoose }: { person: Person; accounts: readonly Account[]; onChoose: Props["onChoose"] }) {
  const { l10n } = useLocalization();
  const value = valueOf(person.destination, accounts);
  const suggested = accounts.find((a) => a.id === person.suggestion);
  const waiting = person.state === "waiting-for-owner";

  return (
    <tr className="people-person">
      <td>
        <Label id="people-col-old" />
        <span className="people-head">
          <strong className="people-name">
            <bdi>{shownName(person.name)}</bdi>
          </strong>
          {person.isYou && (
            <Badge tone="neutral">
              <Localized id="people-you">
                <span />
              </Localized>
            </Badge>
          )}
        </span>
      </td>

      <td>
        <Label id="people-col-new" />
        {waiting ? (
          <Localized id="people-waiting-body" vars={{ name: person.name }}>
            <p className="people-note" />
          </Localized>
        ) : (
          <>
            <Localized id="people-select" attrs={{ "aria-label": true }} vars={{ name: person.name }}>
              <select
                className="kit-select people-select"
                value={value}
                onChange={(e) => {
                  onChoose(person.id, destinationOf(e.target.value));
                }}
              >
                <option value="undecided">{l10n.getString("people-option-choose")}</option>
                {accounts.map((a) => (
                  <option key={a.id} value={`existing:${a.id}`}>
                    {l10n.getString("people-option-existing", { account: a.name })}
                  </option>
                ))}
                <option value="new">{l10n.getString("people-option-new")}</option>
                <option value="leave-out">{l10n.getString("people-option-leave-out")}</option>
              </select>
            </Localized>
            {value === "new" && (
              <Localized id="people-new-account-hint">
                <p className="people-note" />
              </Localized>
            )}
            {value === "undecided" && suggested && (
              <div className="people-suggestion">
                <Localized id="people-suggested">
                  <p className="people-note" />
                </Localized>
                <Button
                  variant="quiet"
                  onClick={() => {
                    onChoose(person.id, { kind: "existing", accountId: suggested.id });
                  }}
                >
                  {l10n.getString("people-suggested-use", { account: suggested.name })}
                </Button>
              </div>
            )}
          </>
        )}
      </td>

      <td>
        <Label id="people-col-status" />
        <div className="people-statuses">
          {waiting ? (
            <Status id="people-status-waiting" tone="warn" icon={<Attention />} />
          ) : value === "undecided" ? (
            <Status id="people-status-choose" tone="warn" icon={<Attention />} />
          ) : value === "leave-out" ? (
            <Status id="people-status-left-out" tone="neutral" icon={<Info />} />
          ) : (
            <Status id="people-status-ready" tone="good" icon={<Check />} />
          )}
          {!waiting && person.changedByYou && <Status id="people-status-changed" tone="neutral" icon={<Info />} />}
        </div>
      </td>
    </tr>
  );
}
