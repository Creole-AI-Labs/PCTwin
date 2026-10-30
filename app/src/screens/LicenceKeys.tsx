import { Localized, useLocalization } from "@fluent/react";
import { useEffect, useRef, useState } from "react";
import type { KeyPlace } from "../bridge";
import { Button } from "../components/kit";

type Props = {
  keys: readonly KeyPlace[];
  /** Reads one key again for the person who asked; null when it cannot be read. */
  onShow: (id: string) => Promise<string | null>;
};

/**
 * Licence keys PCTwin found on this laptop (Task List 2.1, product key finder): only keys an app
 * keeps in the open for this person, shown one at a time when the person asks, on this laptop only.
 * A key is held on this page while it is shown and gone when hidden or when the page closes; it is
 * never put in the move, its report or a log.
 */
export default function LicenceKeys({ keys, onShow }: Props) {
  const { l10n } = useLocalization();
  // Keys shown, held outside React state so they can be dropped when the page closes.
  const shown = useRef(new Map<string, string | null>());
  const [, redraw] = useState(0);
  useEffect(() => {
    const keysShown = shown.current;
    return () => {
      keysShown.clear();
    };
  }, []);

  if (keys.length === 0) return null;
  return (
    <section className="before-keys" aria-labelledby="before-keys-title">
      <Localized id="before-keys-title">
        <h2 id="before-keys-title" className="before-keys-title" />
      </Localized>
      <Localized id="before-keys-body">
        <p className="kit-hint" />
      </Localized>
      <ul className="before-keys-list">
        {keys.map((k) => {
          const value = shown.current.get(k.id) ?? null;
          const open = shown.current.has(k.id);
          return (
            <li key={k.id}>
              <strong>{k.app}</strong>
              {open && value !== null && <code className="before-key">{value}</code>}
              {open && value === null && (
                <Localized id="before-keys-gone" vars={{ app: k.app }}>
                  <small className="kit-hint" role="status" />
                </Localized>
              )}
              {open ? (
                <Button
                  variant="quiet"
                  onClick={() => {
                    shown.current.delete(k.id);
                    redraw((n) => n + 1);
                  }}
                >
                  {l10n.getString("before-keys-hide")}
                </Button>
              ) : (
                <Button
                  variant="quiet"
                  aria-label={l10n.getString("before-keys-show-label", { app: k.app })}
                  onClick={() => {
                    void onShow(k.id)
                      .catch(() => null)
                      .then((key) => {
                        shown.current.set(k.id, key);
                        redraw((n) => n + 1);
                      });
                  }}
                >
                  {l10n.getString("before-keys-show")}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
