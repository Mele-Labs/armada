import { describe, expect, it } from "vitest";

import { POWER_FILL_MS, POWER_GRACE_MS, POWER_LEAST_MS, nextPowerAt, powerOf, type PowerClock } from "./boot";

const OPENED = 1_000_000;
const opened: PowerClock = { opened: OPENED, dropped: null, since: null };

describe("powerOf", () => {
  it("stands by while Bridge reads the runtime file, connects, or Fleet starts, however long it takes", () => {
    for (const state of ["reading", "connecting", "starting"] as const) {
      expect(powerOf(state, OPENED, opened)).toBe("standby");
      expect(powerOf(state, OPENED + POWER_GRACE_MS * 10, opened)).toBe("standby");
    }
  });

  it("stands by for a Fleet that is down inside the grace from Bridge opening, and is a fault after it", () => {
    for (const state of ["not_running", "unreachable"] as const) {
      expect(powerOf(state, OPENED + POWER_GRACE_MS - 1, opened)).toBe("standby");
      expect(powerOf(state, OPENED + POWER_GRACE_MS, opened)).toBe("fault");
    }
  });

  it("counts the grace from the last drop, so a restart long after opening stands by", () => {
    const dropped = OPENED + 10 * POWER_GRACE_MS;
    const clock: PowerClock = { opened: OPENED, dropped, since: null };
    expect(powerOf("unreachable", dropped + POWER_GRACE_MS - 1, clock)).toBe("standby");
    expect(powerOf("not_running", dropped + POWER_GRACE_MS, clock)).toBe("fault");
  });

  it("is a fault at once for a refused runtime file or a protocol mismatch", () => {
    expect(powerOf("runtime_file_refused", OPENED, opened)).toBe("fault");
    expect(powerOf("protocol_mismatch", OPENED, opened)).toBe("fault");
  });

  it("stands by for the least power-on on a launch where Fleet answers at once", () => {
    const clock: PowerClock = { opened: OPENED, dropped: null, since: OPENED };
    expect(powerOf("connected", OPENED, clock)).toBe("standby");
    expect(powerOf("connected", OPENED + POWER_LEAST_MS - 1, clock)).toBe("standby");
    expect(powerOf("connected", OPENED + POWER_LEAST_MS, clock)).toBe("filling");
  });

  it("fills once Fleet answers and is live after the fill", () => {
    const since = OPENED + 5 * POWER_LEAST_MS;
    const clock: PowerClock = { opened: OPENED, dropped: null, since };
    expect(powerOf("connected", since, clock)).toBe("filling");
    expect(powerOf("connected", since + POWER_FILL_MS - 1, clock)).toBe("filling");
    expect(powerOf("connected", since + POWER_FILL_MS, clock)).toBe("live");
  });
});

describe("nextPowerAt", () => {
  it("is the end of the least power-on, then the end of the fill, for a connection", () => {
    const clock: PowerClock = { opened: OPENED, dropped: null, since: OPENED };
    expect(nextPowerAt("connected", OPENED, clock)).toBe(OPENED + POWER_LEAST_MS);
    expect(nextPowerAt("connected", OPENED + POWER_LEAST_MS, clock)).toBe(OPENED + POWER_LEAST_MS + POWER_FILL_MS);
    expect(nextPowerAt("connected", OPENED + POWER_LEAST_MS + POWER_FILL_MS, clock)).toBeUndefined();
  });

  it("is the end of the grace for a Fleet that is down, and nothing past it", () => {
    expect(nextPowerAt("not_running", OPENED, opened)).toBe(OPENED + POWER_GRACE_MS);
    expect(nextPowerAt("unreachable", OPENED + POWER_GRACE_MS, opened)).toBeUndefined();
  });

  it("is nothing where only Fleet can move the reading", () => {
    expect(nextPowerAt("starting", OPENED, opened)).toBeUndefined();
    expect(nextPowerAt("protocol_mismatch", OPENED, opened)).toBeUndefined();
  });
});

describe("a launch on the picture kept on disk", () => {
  const kept: PowerClock = { opened: OPENED, dropped: null, since: null, kept: true };

  it("never stands by or fills: not answering is kept, answering is live at once", () => {
    for (const state of ["reading", "connecting", "starting", "not_running", "unreachable"] as const) {
      expect(powerOf(state, OPENED, kept)).toBe("kept");
    }
    expect(powerOf("connected", OPENED, { ...kept, since: OPENED })).toBe("live");
    expect(nextPowerAt("connected", OPENED, { ...kept, since: OPENED })).toBeUndefined();
  });

  it("is a fault once a Fleet that is down has been silent past the grace", () => {
    expect(powerOf("not_running", OPENED + POWER_GRACE_MS - 1, kept)).toBe("kept");
    expect(powerOf("not_running", OPENED + POWER_GRACE_MS, kept)).toBe("fault");
  });
});
