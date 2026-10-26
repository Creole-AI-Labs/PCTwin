import { Localized, useLocalization } from "@fluent/react";
import { useState, type SyntheticEvent } from "react";
import { Button, TextField } from "../components/kit";
import { Laptop } from "../icons";
import { labelText, type Label } from "../pairing";
import type { Problem } from "../bridge";
import FrameFooter from "./FrameFooter";
import { Heading, ProblemNotice } from "./PairParts";

/**
 * Name this laptop (decided 26 October 2026): the first time PCTwin opens, the name other laptops
 * nearby will find this one by. It suggests the laptop's own name from its settings, or a random
 * colour and animal when that name was made up by the factory; the person keeps it, chooses
 * another, or types one. Anyone on the same Wi-Fi can see the name, and the page says so.
 * Events: onUse (keep the name shown), onShuffle, onType(name), each answered by the bridge.
 */
export default function NameLaptop({
  suggested,
  problem,
  onUse,
  onShuffle,
  onType,
}: {
  suggested: Label;
  /** The last thing that went wrong (a name that cannot be used, say), or null. */
  problem: Problem | null;
  onUse: () => void;
  onShuffle: () => void;
  onType: (name: string) => Promise<boolean>;
}) {
  const { l10n } = useLocalization();
  const [typing, setTyping] = useState(false);
  const [name, setName] = useState("");
  const submit = async (e: SyntheticEvent) => {
    e.preventDefault();
    if (await onType(name.trim())) setName("");
  };
  return (
    <>
      <Heading id="name-title" />
      <Localized id="name-body">
        <p className="subhead" />
      </Localized>
      <section className="pair-card" aria-labelledby="name-shown">
        <p className="pair-label-name" id="name-shown" dir="auto">
          <Laptop />
          <span>{labelText(l10n, suggested)}</span>
        </p>
        <Localized id="pair-old-rename-note">
          <p className="pair-card-hint" />
        </Localized>
        {typing ? (
          <form className="pair-rename" onSubmit={(e) => void submit(e)}>
            <TextField
              label={
                <Localized id="pair-old-rename-label">
                  <span />
                </Localized>
              }
              value={name}
              onChange={(e) => {
                setName(e.target.value);
              }}
              autoComplete="off"
              spellCheck={false}
              autoFocus
            />
            <div className="pair-actions">
              <Button type="submit" variant="primary">
                <Localized id="pair-old-rename-save">
                  <span />
                </Localized>
              </Button>
              <Button
                onClick={() => {
                  setTyping(false);
                  setName("");
                }}
              >
                <Localized id="pair-old-rename-cancel">
                  <span />
                </Localized>
              </Button>
            </div>
          </form>
        ) : (
          <div className="pair-actions">
            <Button onClick={onShuffle}>
              <Localized id="pair-old-shuffle">
                <span />
              </Localized>
            </Button>
            <button
              type="button"
              className="kit-button kit-button-quiet"
              onClick={() => {
                setTyping(true);
              }}
            >
              <Localized id="pair-old-rename">
                <span />
              </Localized>
            </button>
          </div>
        )}
      </section>
      {problem && <ProblemNotice problem={problem} />}
      {!typing && (
        <FrameFooter>
          <Button variant="primary" onClick={onUse}>
            <Localized id="name-use">
              <span />
            </Localized>
          </Button>
        </FrameFooter>
      )}
    </>
  );
}
