import { Localized, useLocalization } from "@fluent/react";
import { PREFERENCES, useMotion, type MotionPreference } from "./preference";

/**
 * The in-app Motion choice: follow the system (the default), reduced, or off. It sits under the
 * language choice, in the panel the header's language control opens (the app's only settings place
 * for now). Changing it applies at once.
 */
export default function MotionChoice() {
  const { l10n } = useLocalization();
  const { preference, setPreference } = useMotion();
  return (
    <label className="motion-choice">
      <Localized id="motion-label">
        <span className="visually-hidden">Motion</span>
      </Localized>
      <select
        value={preference}
        onChange={(e) => {
          setPreference(e.target.value as MotionPreference);
        }}
      >
        {PREFERENCES.map((p) => (
          <option key={p} value={p}>
            {l10n.getString(`motion-${p}`)}
          </option>
        ))}
      </select>
    </label>
  );
}
