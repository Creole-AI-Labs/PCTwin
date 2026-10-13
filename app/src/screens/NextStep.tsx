import { Localized } from "@fluent/react";
import { ArrowBack } from "../icons";
import type { Role } from "./WhichLaptop";

/** Where pairing (C03/C04) will start. Until it is built, it says so plainly and changes nothing. */
export default function NextStep({ role, onBack }: { role: Role; onBack: () => void }) {
  return (
    <>
      <button type="button" className="link back" onClick={onBack}>
        <ArrowBack />
        <Localized id="role-back">
          <span>Back</span>
        </Localized>
      </button>
      <Localized id={role === "old" ? "next-title-old" : "next-title-new"}>
        <h1>{role === "old" ? "This is the old laptop" : "This is the new laptop"}</h1>
      </Localized>
      <Localized id="next-body">
        <p className="lead">Pairing the two laptops comes next. It is still being built, so nothing has been changed on this laptop.</p>
      </Localized>
    </>
  );
}
