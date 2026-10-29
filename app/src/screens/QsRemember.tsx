import { useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import { suggestReplacement, type QsRefusal, type RememberChoice } from "../bridge";
import { Banner, Button, TextField } from "../components/kit";
import { dateText } from "./FinishParts";
import { Name } from "./QsParts";

const KEEP_BOTH = "";

/**
 * Q21 Remember this connection, offered after a send, a receive or a note over the code. The
 * person names it; when it may be a laptop remembered before (pairing again makes fresh keys, so
 * only the person can say), the earlier connections are listed to choose the one this replaces,
 * with "No, keep both" first. One whose name matches the typed name is chosen already and can be
 * changed; once the person chooses, typing no longer moves the choice. Remembering never allows
 * anything to arrive unasked. `remembered`: the name it was remembered as.
 * Events: onRemember(label, replaces).
 */
export function RememberOffer({
  fallbackStore,
  connections,
  suggestedName = "",
  problem = null,
  remembered = null,
  onRemember,
}: {
  fallbackStore: boolean;
  connections: RememberChoice[];
  suggestedName?: string;
  problem?: QsRefusal | null;
  remembered?: string | null;
  onRemember: (label: string, replaces: string | null) => void;
}) {
  const { l10n } = useLocalization();
  const titleId = useId();
  const [name, setName] = useState(suggestedName);
  // The person's own choice of what this replaces; until then, the suggestion follows the name.
  const [chosen, setChosen] = useState<string | null>(null);
  const suggested = suggestReplacement(connections, name);
  const replaces = chosen ?? suggested?.id ?? KEEP_BOTH;

  if (remembered !== null)
    return (
      <section className="qs-card qs-remember" aria-labelledby={titleId}>
        <h2 id={titleId} className="qs-card-title">
          {l10n.getString("qs-remember-ask")}
        </h2>
        <p role="status">{l10n.getString("qs-remember-done", { label: remembered })}</p>
      </section>
    );

  return (
    <section className="qs-card qs-remember" aria-labelledby={titleId}>
      <h2 id={titleId} className="qs-card-title">
        {l10n.getString("qs-remember-ask")}
      </h2>
      <p className="qs-small">{l10n.getString("qs-remember-body")}</p>
      {fallbackStore && <p className="qs-small">{l10n.getString("qs-remember-fallback")}</p>}
      <TextField
        label={l10n.getString("qs-remember-name")}
        hint={l10n.getString("qs-remember-name-hint")}
        value={name}
        dir="auto"
        maxLength={40}
        onChange={(e) => {
          setName(e.target.value);
        }}
      />
      {connections.length > 0 && (
        <fieldset className="qs-choice-list">
          <legend>{l10n.getString("qs-remember-replaces")}</legend>
          {suggested && chosen === null && (
            <p className="qs-small" role="status">
              {l10n.getString("qs-remember-replaces-suggested", { label: suggested.label })}
            </p>
          )}
          <label className="kit-check">
            <input
              type="radio"
              name={`${titleId}-replaces`}
              checked={replaces === KEEP_BOTH}
              onChange={() => {
                setChosen(KEEP_BOTH);
              }}
            />
            <span>
              <strong>{l10n.getString("qs-remember-keep-both")}</strong>
            </span>
          </label>
          {connections.map((c) => (
            <label key={c.id} className="kit-check">
              <input
                type="radio"
                name={`${titleId}-replaces`}
                checked={replaces === c.id}
                onChange={() => {
                  setChosen(c.id);
                }}
              />
              <span>
                <strong>
                  <Name>{c.label}</Name>
                </strong>
                <small className="kit-hint">{l10n.getString("qs-known-last-used", { date: dateText(c.lastUsed) })}</small>
              </span>
            </label>
          ))}
        </fieldset>
      )}
      {problem && <Banner tone="attention">{l10n.getString(`qs-refusal-${problem.kind}`)}</Banner>}
      <div className="qs-actions">
        <Button
          disabled={name.trim() === ""}
          onClick={() => {
            onRemember(name.trim(), replaces === KEEP_BOTH ? null : replaces);
          }}
        >
          {l10n.getString(replaces === KEEP_BOTH ? "qs-remember-save" : "qs-remember-replace")}
        </Button>
      </div>
    </section>
  );
}
