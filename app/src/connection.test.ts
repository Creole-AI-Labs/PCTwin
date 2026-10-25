import { describe, expect, it } from "vitest";
import { connectionCheck, peerReadiness, type Greeted, type Naming } from "./connection";

const naming: Naming = { system: (s) => ({ windows: "Windows", mac: "Mac", linux: "Linux", other: "Another system" })[s], thisLaptop: "This laptop", otherLaptop: "The other laptop" };

const greeted = (over: Partial<Greeted> = {}): Greeted => ({
  mine: { system: "mac", chip: "aarch64", app: "0.1.0", power: { kind: "plugged" }, freeBytes: 410_000_000_000 },
  theirs: { system: "windows", chip: "x86_64", app: "0.1.0", power: { kind: "battery", percent: 34 }, freeBytes: 52_000_000_000 },
  otherLabel: "Blue Heron",
  versions: "same",
  link: { bytesPerSecond: 62_400_000, slow: false },
  ...over,
});

describe("C06 from the greeting", () => {
  it("the old laptop sends and the new one receives, whichever this is", () => {
    const onNew = peerReadiness(greeted(), false, naming);
    expect(onNew.state).toBe("ready");
    expect(onNew.sending).toEqual({ label: "Blue Heron", system: "Windows · x86_64", version: "0.1.0" });
    expect(onNew.receiving.label).toBe("This laptop");
    const onOld = peerReadiness(greeted(), true, naming);
    expect(onOld.sending.label).toBe("This laptop");
    expect(onOld.receiving.system).toBe("Windows · x86_64");
  });

  it("names the laptop to update: this one or the other, on either side", () => {
    const at = (versions: Greeted["versions"], old: boolean) => {
      const r = peerReadiness(greeted({ versions }), old, naming);
      return r.state === "versions-differ" ? r.updateOn : null;
    };
    expect(at("update-here", true)).toBe("sending");
    expect(at("update-here", false)).toBe("receiving");
    expect(at("update-there", true)).toBe("receiving");
    expect(at("update-there", false)).toBe("sending");
  });

  it("a system PCTwin does not move is said so", () => {
    const other = greeted({ theirs: { ...greeted().theirs, system: "other" } });
    expect(peerReadiness(other, false, naming).state).toBe("direction-unsupported");
  });

  it("no pairing label: the other laptop is called so", () => {
    expect(peerReadiness(greeted({ otherLabel: null }), false, naming).sending.label).toBe("The other laptop");
  });
});

describe("C07 from the greeting", () => {
  it("the measured speed, the new laptop's free space, power on both; the internet is never checked", () => {
    const c = connectionCheck(greeted(), false, naming);
    expect(c).toEqual({
      oldLabel: "Blue Heron",
      newLabel: "This laptop",
      speedMbPerSecond: 62,
      link: "good",
      freeSpaceGb: 410,
      powerOld: "on-battery",
      powerNew: "plugged-in",
      internet: "unknown",
    });
    // On the old laptop, the free space is still the new laptop's.
    expect(connectionCheck(greeted(), true, naming).freeSpaceGb).toBe(52);
  });

  it("what could not be read stays unknown, never fine", () => {
    const c = connectionCheck(greeted({ link: null, mine: { ...greeted().mine, freeBytes: null, power: { kind: "unknown" } } }), false, naming);
    expect(c.speedMbPerSecond).toBeNull();
    expect(c.freeSpaceGb).toBeNull();
    expect(c.powerNew).toBe("unknown");
  });

  it("a slow link is weak", () => {
    expect(connectionCheck(greeted({ link: { bytesPerSecond: 1_200_000, slow: true } }), false, naming).link).toBe("weak");
  });
});
