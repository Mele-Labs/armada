// What the duration formatters say, at the boundaries where they change words.
import { describe, expect, it } from "vitest";

import { briefly, clock, clockOf, elapsedSince, lasting, span } from "./duration";

describe("a short duration keeps its milliseconds", () => {
  it("reports under a second in milliseconds", () => {
    expect(briefly(200)).toBe("200ms");
  });

  it("hands a second and over to the same formatter every other span uses", () => {
    expect(briefly(1_000)).toBe("1s");
    expect(briefly(142_000)).toBe(lasting(142_000));
  });

  it("never reports a negative duration", () => {
    expect(briefly(-5)).toBe("0ms");
  });

  it("is why the body does not use `lasting` for a call", () => {
    // 343 of the recorded step's 351 calls answered inside a second, and
    // `lasting` rounds every one of them to the same word.
    expect(lasting(200)).toBe("0s");
    expect(briefly(200)).not.toBe(lasting(200));
  });
});

describe("a span between two instants", () => {
  it("says nothing where an end will not parse", () => {
    expect(span("2026-09-10T14:00:00Z", "not a date")).toBeNull();
  });

  it("reads minutes and seconds", () => {
    expect(span("2026-09-10T14:00:00Z", "2026-09-10T14:02:22Z")).toBe("2m 22s");
  });
});

describe("a whole-Job elapsed, from started_at", () => {
  it("is undefined for a Job that has never run", () => {
    expect(elapsedSince(undefined, "2026-09-10T14:02:22Z")).toBeUndefined();
  });

  it("spans from started_at once the Job has run", () => {
    expect(elapsedSince("2026-09-10T14:00:00Z", "2026-09-10T14:02:22Z")).toBe("2m 22s");
  });

  it("takes `now` as a number the same way span does", () => {
    const now = Date.parse("2026-09-10T14:02:22Z");
    expect(elapsedSince("2026-09-10T14:00:00Z", now)).toBe("2m 22s");
  });
});

describe("a wall clock", () => {
  const AT = "2026-10-07T13:48:02Z";

  it("reads as the platform's own time formatting did, from one shared formatter", () => {
    expect(clock(AT)).toBe(new Date(AT).toLocaleTimeString([], { hour12: false }));
    expect(clockOf(AT)).toBe(new Date(AT).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }));
  });

  it("gives back what it cannot read", () => {
    expect(clock("soon")).toBe("soon");
  });
});
