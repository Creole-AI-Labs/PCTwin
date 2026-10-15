/**
 * View models for the connection screens (lane A): the resume banner on Welcome (F01), and
 * C06 Meet your other laptop, C07 Let's check the connection, C09 Waiting for your other laptop and
 * C10 The connection has ended. Screens take these as props and report choices as callbacks; the
 * bridge to the engine fills them later. C01 and C02 need no engine data and have no model here.
 *
 * Engine commands and events these screens need (none exist yet; pairing's commands are in pairing.ts).
 * Names are proposals for the bridge; payloads are the shapes below.
 *
 *  F01  command `unfinished_move() -> UnfinishedMove | null`      asked once when Welcome opens.
 *       command `resume_move()`                                    when "Open this move" is chosen; the engine
 *                                                                  reconnects (C09) or goes to the move.
 *  C06  event   `peer-readiness` { PeerReadiness }                 once, right after pairing succeeds and the
 *                                                                  two laptops have exchanged their details.
 *  C07  command `connection_check() -> ConnectionCheck`           when C07 opens (the screen shows "measuring"
 *                                                                  until it answers) and again on "Check again".
 *                                                                  Measures speed with a short real transfer,
 *                                                                  reads free space on the new laptop, power on
 *                                                                  both, and whether the internet is reachable.
 *                                                                  Anything it could not read is null / "unknown".
 *  C09  event   `link-lost` { WaitingState }                       when the connection drops mid-move; repeated
 *                                                                  whenever saved progress or last-seen change
 *                                                                  (at most once a second), and with a new kind
 *                                                                  when retries give up (unreachable) or the
 *                                                                  resume ticket expires (pairing-expired).
 *       event   `link-restored` {}                                 the screen closes and the move carries on.
 *       command `retry_connection()`                               "Try now".
 *  C10  event   `peer-left` { movedSomething: boolean }            the other laptop closed or left on purpose.
 *       command `pair_again()`                                     starts a fresh pairing for the same saved move.
 *       command `open_saved_move()`                                opens the report of what was already moved.
 *
 * Every count and size below is measured by the engine; nothing here is estimated or animated.
 */

/** One laptop as the other one sees it. `label` is the readable name (colour and animal, or the person's own). */
export interface Laptop {
  label: string;
  /** For example "Windows 11 · x64". A proper name, shown as given. */
  system: string;
  /** The PCTwin version, for example "1.4.2". */
  version: string;
}

/** F01: a move that was paused or stopped before it finished, still saved on this laptop. */
export interface UnfinishedMove {
  /** The other laptop's label. */
  otherLaptop: string;
  /** `paused`: the person paused it. `interrupted`: it stopped by itself (power, crash, connection). */
  state: "paused" | "interrupted";
  /** When it stopped. */
  stoppedAt: Date;
}

/** C06: which of the three things the two laptops' details add up to. */
export type PeerReadiness =
  | { state: "ready"; sending: Laptop; receiving: Laptop }
  /** `updateOn`: the laptop with the older version, which the engine decides; it is the one to update. */
  | { state: "versions-differ"; sending: Laptop; receiving: Laptop; updateOn: "sending" | "receiving" }
  | { state: "direction-unsupported"; sending: Laptop; receiving: Laptop };

export type PowerState = "plugged-in" | "on-battery" | "unknown";

/**
 * C07: what was measured. `null` or "unknown" means it could not be read, and the screen says so
 * instead of showing it as fine. While the engine is still measuring, the screen gets no check at all.
 */
export interface ConnectionCheck {
  /** The readable names of the two laptops, for the power lines. */
  oldLabel: string;
  newLabel: string;
  /** Measured transfer speed between the two laptops, megabytes a second. */
  speedMbPerSecond: number | null;
  /** The engine's call on that speed. `weak` is slow enough that a move will take noticeably longer. */
  link: "good" | "weak";
  /** Free space on the new laptop, in gigabytes (1 GB = 1,000,000,000 bytes). */
  freeSpaceGb: number | null;
  powerOld: PowerState;
  powerNew: PowerState;
  /** Never needed for copying files between the laptops. */
  internet: "online" | "offline" | "unknown";
}

/** C09: why the move is waiting. */
export type WaitingKind = "reconnecting" | "pairing-expired" | "unreachable";

export interface WaitingState {
  kind: WaitingKind;
  /** The other laptop's readable name. */
  otherLaptop: string;
  /** Copied and checked so far (measured), in gigabytes, and the total chosen for the move. */
  savedGb: number;
  totalGb: number;
  /** Seconds since the other laptop was last heard from; null if it was not heard from this time (the line is left out). */
  lastSeenSeconds: number | null;
}
