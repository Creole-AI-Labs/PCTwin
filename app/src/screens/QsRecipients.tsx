import { useLocalization } from "@fluent/react";
import type { KnownConnection, QsRefusal } from "../bridge";
import { Banner, Button, Checkbox, Segmented, Select } from "../components/kit";
import { dateText } from "./FinishParts";
import FrameFooter from "./FrameFooter";
import { DEVICE_LINE } from "./QsKnown";
import { Heading, Name, QsActions, QsStrip } from "./QsParts";

/** How long a waiting send may wait, in days (the engine takes 1 to 30; seven is the default). */
export const WAIT_DAYS = ["1", "3", "7", "14", "30"] as const;
export type WaitDays = (typeof WAIT_DAYS)[number];
export type When = "now" | "later";

/**
 * Q04 and Q17 together: who gets the files chosen on the pick page. Remembered laptops are ticked
 * (several in one approval), or "Use a code instead" pairs a laptop with the code as before. Now
 * sends while the laptops are nearby: up to three at the same time, the others waiting their turn
 * (29 October 2026). When it is nearby makes a waiting send: "Approve this delivery" covers these files as they are now, these laptops and how
 * long it waits, and PCTwin keeps a copy to send. Each laptop is still asked where to save, unless
 * it allowed sends into a folder.
 * Events: onToggle(id, on), onWhen, onDays, onCode, onSendNow, onApprove, onBack.
 */
export default function QsRecipients({
  connections,
  chosen,
  when,
  days,
  problem = null,
  busy = false,
  onToggle,
  onWhen,
  onDays,
  onCode,
  onSendNow,
  onApprove,
  onBack,
}: {
  connections: KnownConnection[];
  chosen: string[];
  when: When;
  days: WaitDays;
  problem?: QsRefusal | null;
  /** A command is running: the main action waits. */
  busy?: boolean;
  onToggle: (id: string, on: boolean) => void;
  onWhen: (when: When) => void;
  onDays: (days: WaitDays) => void;
  onCode: () => void;
  onSendNow: () => void;
  onApprove: () => void;
  onBack: () => void;
}) {
  const { l10n } = useLocalization();
  const one = chosen.length === 1;
  const now = when === "now";
  return (
    <>
      <QsStrip onBack={onBack} />
      <Heading id="qs-to-title" />
      <p className="subhead">{l10n.getString("qs-to-intro")}</p>

      <fieldset className="qs-choice-list">
        <legend>{l10n.getString("qs-to-known")}</legend>
        <ul className="qs-offer">
          {connections.map((c) => (
            <li key={c.id}>
              <Checkbox
                checked={chosen.includes(c.id)}
                onChange={(on) => {
                  onToggle(c.id, on);
                }}
                label={<Name>{c.label}</Name>}
                hint={`${l10n.getString("qs-known-last-used", { date: dateText(c.lastUsed) })} · ${l10n.getString(DEVICE_LINE[c.device])}`}
              />
            </li>
          ))}
        </ul>
      </fieldset>

      <QsActions>
        <Button onClick={onCode}>{l10n.getString("qs-to-code")}</Button>
      </QsActions>

      {chosen.length > 0 && (
        <section className="qs-card" aria-label={l10n.getString("qs-to-when")}>
          <Segmented
            label={l10n.getString("qs-to-when")}
            hint={now ? l10n.getString(one ? "qs-to-now-hint" : "qs-to-now-several-hint") : one ? l10n.getString("qs-waiting-on") : `${l10n.getString("qs-to-several")} ${l10n.getString("qs-waiting-on")}`}
            value={when}
            options={[
              { value: "now", label: l10n.getString(one ? "qs-to-now" : "qs-to-now-several") },
              { value: "later", label: l10n.getString("qs-to-later") },
            ]}
            onChange={onWhen}
          />
          {!now && (
            <Select
              label={l10n.getString("qs-to-days")}
              value={days}
              options={WAIT_DAYS.map((d) => ({ value: d, label: l10n.getString("qs-waiting-days", { count: Number(d) }) }))}
              onChange={onDays}
            />
          )}
          <p className="qs-small">{l10n.getString(now ? "qs-to-each-asks" : "qs-to-approve-hint")}</p>
        </section>
      )}

      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}

      <FrameFooter note="send">
        {now ? (
          <Button variant="primary" disabled={busy || chosen.length === 0} onClick={onSendNow}>
            {l10n.getString("qs-to-send-now")}
          </Button>
        ) : (
          <Button variant="primary" disabled={busy || chosen.length === 0} onClick={onApprove}>
            {l10n.getString("qs-to-approve")}
          </Button>
        )}
      </FrameFooter>
    </>
  );
}
