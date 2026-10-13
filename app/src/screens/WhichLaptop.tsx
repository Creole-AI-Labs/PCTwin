import { Localized } from "@fluent/react";
import { ArrowBack, ArrowForward, Info, Laptop } from "../icons";

export type Role = "old" | "new";

/** F06 Old or new laptop: the person chooses; PCTwin never decides from the laptop's age. */
export default function WhichLaptop({ onBack, onChoose }: { onBack: () => void; onChoose: (role: Role) => void }) {
  return (
    <>
      <button type="button" className="link back" onClick={onBack}>
        <ArrowBack />
        <Localized id="role-back">
          <span>Back</span>
        </Localized>
      </button>
      <Localized id="role-eyebrow">
        <p className="eyebrow">Start a move</p>
      </Localized>
      <Localized id="role-title">
        <h1>Which laptop is this?</h1>
      </Localized>
      <Localized id="role-body">
        <p className="lead">Open PCTwin on both laptops. Choose the role for this one.</p>
      </Localized>
      <ul className="choices">
        <li>
          <Choice role="old" onChoose={onChoose} />
        </li>
        <li>
          <Choice role="new" onChoose={onChoose} />
        </li>
      </ul>
      <div className="note">
        <Info />
        <Localized id="role-note">
          <p className="note-text">Nothing moves until you review and approve the plan. Your old laptop stays unchanged during the move.</p>
        </Localized>
      </div>
    </>
  );
}

function Choice({ role, onChoose }: { role: Role; onChoose: (role: Role) => void }) {
  return (
    <button type="button" className="choice" onClick={() => { onChoose(role); }}>
      <span className="choice-icon">
        <Laptop />
      </span>
      <span className="choice-text">
        <Localized id={`role-${role}-title`}>
          <span className="choice-title">{role === "old" ? "My old laptop" : "My new laptop"}</span>
        </Localized>
        <Localized id={`role-${role}-body`}>
          <span className="choice-body">{role === "old" ? "Send my files, apps and settings." : "Receive my setup from another laptop."}</span>
        </Localized>
      </span>
      <ArrowForward />
    </button>
  );
}
