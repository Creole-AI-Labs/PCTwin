/**
 * View models for the people-and-help screens: Who's moving? (M06), Before you start (M10/M13 early)
 * and Get help. None has engine data yet, so each screen takes props in and sends events out; the
 * bridge fills them later. Nothing here calls a command that does not exist.
 *
 * Commands and events the bridge will need (none exist yet):
 *
 * M06 Who's moving?
 *   command `people_list` ()                       -> { people: Person[], accounts: Account[], shared: SharedHome }
 *     The profiles found on the old laptop, the accounts that already exist on the new laptop, and
 *     the current choice for shared items. Other people's profiles arrive as `waiting-for-owner`.
 *   event   `people-changed` { people: Person[] }  -> when an owner signs in on the old laptop and
 *     approves (a person's `state` becomes `ready`), or when the list of accounts changes.
 *   command `people_set_destination` ({ personId, destination: Destination })
 *     Stores the person's choice in the plan. Emitted by `onChoose`. A suggestion is never stored by itself.
 *   command `people_set_shared` ({ shared: SharedHome })     Emitted by `onChooseShared`.
 *
 * Before you start
 *   command `before_list` ()                        -> { items: BeforeItem[] }
 *     The things found in the plan that cannot move by copying (licensed apps, browser sign-ins).
 *   command `before_set_done` ({ id, done: boolean })
 *     Records the person's own tick. PCTwin cannot see whether it was really done, so it is stored
 *     as "you said you did this", never as something PCTwin measured. Emitted by `onToggle`.
 *
 * Get help
 *   command `help_make_code` ()                     -> { code: string, included: HelpIncluded[] }
 *     While it runs the screen shows `{ status: "making" }`.
 *     Builds a short problem code. It holds no names, file names, folder paths or network names.
 *     Rejects when it cannot (the screen shows "could not make a code"). Emitted by `onRetry`.
 *   command `help_share` ({ code: string, message: string })
 *     Opens the system share sheet (WhatsApp or any app) with `message` (the help-share-text line, which holds the code). Emitted
 *     by `onShare`. The `canShare` prop says whether this laptop has a share sheet.
 *   Copy is done by the screen itself with the system clipboard; it needs no command.
 */

/* ---------- M06 Who's moving? ---------- */

/** An account that already exists on the new laptop. `name` is what the person typed, shown as written. */
export type Account = { id: string; name: string };

/** Where one person's things go on the new laptop. */
export type Destination =
  | { kind: "existing"; accountId: string }
  | { kind: "new" }
  | { kind: "leave-out" }
  /** Nothing chosen yet. A name that matches is only a suggestion, so this stays until the person chooses. */
  | { kind: "undecided" };

export type Person = {
  id: string;
  /** The profile's name on the old laptop, as written. */
  name: string;
  /** This person's own profile, the one signed in on the old laptop. */
  isYou: boolean;
  /**
   * `ready`: the owner is signed in on the old laptop and has approved, so this profile can be mapped.
   * `waiting-for-owner`: not yet; it cannot be chosen, and nothing of it moves until its owner approves.
   */
  state: "ready" | "waiting-for-owner";
  destination: Destination;
  /** An account whose name matches this profile. Only a suggestion: the person decides. */
  suggestion?: string;
  /** The person changed the mapping on this screen after it was first shown. */
  changedByYou?: boolean;
};

/** Where shared items (shared folders, public documents) go on the new laptop. */
/** `person`: with the files of the person signed in (the one whose profile is `isYou`). */
export type SharedHome = "shared" | "person" | "leave-out";

/* ---------- Before you start ---------- */

/** The kinds of thing that cannot move by copying. Each kind has its own words (before-<kind>-title and -body). */
export type BeforeKind = "licences" | "passwords" | "codes" | "online";

export type BeforeItem = {
  id: string;
  kind: BeforeKind;
  /** The person ticked it. PCTwin cannot check it, so it is the person's word and nothing more. */
  done: boolean;
};

/* ---------- Get help ---------- */

/** What the code holds, in kinds. Each kind has its own plain-word line (help-contains-<kind>). */
export type HelpIncluded = "problem" | "step" | "versions";

export type HelpCode =
  /** The engine is still making the code. */
  | { status: "making" }
  | { status: "ready"; code: string; included: HelpIncluded[] }
  | { status: "failed" };
