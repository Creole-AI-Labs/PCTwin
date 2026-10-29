import { useLocalization } from "@fluent/react";
import { useId, useState } from "react";
import { Button, Segmented } from "../components/kit";
import { HELP_URL, openExternal } from "../external";
import { TopStrip } from "./FrameParts";
import { Heading } from "./ProgressParts";
import "./feedback.css";

type Rating = "none" | "very-good" | "good" | "needs-work";
const RATINGS: readonly Rating[] = ["none", "very-good", "good", "needs-work"];
const MAX_WORDS = 1000;

/**
 * Help, then Feedback (R12): an optional rating and optional words, shown back as the exact text
 * that would be shared. PCTwin sends nothing: the person copies the text (Copy says "copied" only
 * once the system clipboard accepted it) and may open getpctwin.com. No file names, no lists, no
 * account; the text holds only what the person chose here.
 */
export default function Feedback({ onBack }: { onBack: () => void }) {
  const { l10n } = useLocalization();
  const wordsId = useId();
  const [rating, setRating] = useState<Rating>("none");
  const [words, setWords] = useState("");
  const [copy, setCopy] = useState<"copied" | "failed" | null>(null);

  const trimmed = words.trim();
  // Exactly what is previewed and exactly what Copy puts on the clipboard.
  const text = [rating === "none" ? "" : l10n.getString("feedback-preview-rating", { rating: l10n.getString(`feedback-rating-${rating}`) }), trimmed].filter(Boolean).join("\n");

  const doCopy = async () => {
    try {
      const clipboard = (navigator as { clipboard?: Clipboard }).clipboard;
      if (!clipboard) throw new Error("no clipboard");
      await clipboard.writeText(text);
      setCopy("copied");
    } catch {
      setCopy("failed");
    }
  };

  return (
    <>
      <TopStrip onBack={onBack} labelId="feedback-strip" />
      <Heading id="feedback-title" />
      <p className="subhead">{l10n.getString("feedback-body")}</p>
      <div className="fb-form">
        <Segmented<Rating>
          label={l10n.getString("feedback-rating-label")}
          value={rating}
          options={RATINGS.map((value) => ({ value, label: l10n.getString(`feedback-rating-${value}`) }))}
          onChange={(value) => {
            setRating(value);
            setCopy(null);
          }}
        />
        <div className="kit-field">
          <label htmlFor={wordsId}>{l10n.getString("feedback-words-label")}</label>
          <textarea
            id={wordsId}
            className="fb-words"
            rows={4}
            maxLength={MAX_WORDS}
            value={words}
            aria-describedby={`${wordsId}-hint`}
            onChange={(e) => {
              setWords(e.target.value);
              setCopy(null);
            }}
          />
          <small id={`${wordsId}-hint`} className="kit-hint">
            {l10n.getString("feedback-words-hint")}
          </small>
        </div>
      </div>
      <section className="fb-preview" aria-labelledby={`${wordsId}-preview`}>
        <h2 id={`${wordsId}-preview`}>{l10n.getString("feedback-preview-title")}</h2>
        {text ? (
          <p className="fb-preview-text" dir="auto">
            {text}
          </p>
        ) : (
          <p className="kit-hint">{l10n.getString("feedback-preview-empty")}</p>
        )}
      </section>
      <p className="kit-hint">{l10n.getString("feedback-sends-nothing")}</p>
      <div className="mv-actions fb-actions">
        <Button variant="primary" disabled={text === ""} onClick={() => void doCopy()}>
          {l10n.getString("feedback-copy")}
        </Button>
        <Button variant="secondary" onClick={() => void openExternal(HELP_URL)}>
          {l10n.getString("feedback-open-site")}
        </Button>
      </div>
      <div role="status">
        {copy === "copied" && <p>{l10n.getString("feedback-copied")}</p>}
        {copy === "failed" && <p>{l10n.getString("feedback-copy-failed")}</p>}
      </div>
    </>
  );
}
