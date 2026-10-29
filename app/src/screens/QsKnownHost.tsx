import { useEffect, useState } from "react";
import {
  knownAllowFolder,
  knownClearNextFolder,
  knownConnections,
  knownForget,
  knownNextFolder,
  knownRename,
  knownRenewFolder,
  knownRevokeFolder,
  listenKnown,
  toQsRefusal,
  type KnownConnection,
  type QsRefusal,
} from "../bridge";
import KnownList from "./QsKnown";

/**
 * The remembered connections page with the engine: the list asked once and kept current by
 * `known-changed`; each control sends its command and shows the list it returns, or the
 * refusal on that connection's card.
 */
export default function KnownHost({ onSend, onBack }: { onSend: () => void; onBack: () => void }) {
  const [connections, setConnections] = useState<KnownConnection[]>([]);
  const [failed, setFailed] = useState(false);
  const [problem, setProblem] = useState<{ id: string; refusal: QsRefusal } | null>(null);

  useEffect(() => {
    let live = true;
    let unlisten: (() => void) | null = null;
    knownConnections().then(
      (list) => {
        if (live) setConnections(list);
      },
      () => {
        if (live) setFailed(true);
      },
    );
    void listenKnown((event) => {
      if (live && event.kind === "connections") setConnections(event.connections);
    }).then((stop) => {
      if (live) unlisten = stop;
      else stop();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, []);

  const act = (id: string, command: () => Promise<KnownConnection[] | { none: true }>) => {
    setProblem(null);
    command().then(
      (list) => {
        // The picker closed with no folder chosen: nothing changed.
        if (!("none" in list)) setConnections(list);
      },
      (error: unknown) => {
        setProblem({ id, refusal: toQsRefusal(error) });
      },
    );
  };

  return (
    <KnownList
      connections={connections}
      failed={failed}
      problem={problem}
      onSend={onSend}
      onBack={onBack}
      onRename={(id, label) => {
        act(id, () => knownRename(id, label));
      }}
      onForget={(id) => {
        act(id, () => knownForget(id));
      }}
      onAllowFolder={(id) => {
        act(id, () => knownAllowFolder(id));
      }}
      onRevokeFolder={(id) => {
        act(id, () => knownRevokeFolder(id));
      }}
      onRenewFolder={(id) => {
        act(id, () => knownRenewFolder(id));
      }}
      onNextFolder={(id) => {
        act(id, () => knownNextFolder(id));
      }}
      onClearNextFolder={(id) => {
        act(id, () => knownClearNextFolder(id));
      }}
    />
  );
}
