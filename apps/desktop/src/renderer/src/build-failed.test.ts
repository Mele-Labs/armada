// What a Session is told when a failed restart's toast hands it over: which build, the commit, Fleet's line and the log to read.

import { expect, test } from "vitest";
import type { BridgeIdentity } from "@armada/protocol";
import { raisedFailure } from "./failing";
import { failureMessage } from "./failure-message";
import type { WindowFacts } from "../../shared/annotations";

const BRIDGE: BridgeIdentity = { auditPath: "/Users/o/Library/Application Support/Armada/audit.jsonl", fleetProtocol: "32a42be6" };
const VIEW: WindowFacts = { screen: "Cockpit", layer: null, location: "/index.html", scenario: null, window: { width: 1280, height: 800 } };
const LINE = "error[E0432]: unresolved import crate::drones";

function told(build: "main" | "preview" | undefined) {
  const failure = raisedFailure({ key: 0, buildFailed: { ...(build === undefined ? {} : { build }), running: "main", commit: "8944d2fca1b2c3d4e5f6", line: LINE } }, BRIDGE);
  if (failure === null) throw new Error("a failed restart is a failure");
  const report = { on: "main" as const, commit: "8944d2fca1b2c3d4e5f6", failed: LINE };
  return { failure, text: failureMessage({ ...failure.payload, at: "2026-10-17T18:06:07.000Z" }, VIEW, { sessions: { state: "none" }, fleetBuild: report }) };
}

test("the toast names the build and Fleet's line", () => {
  expect(told("preview").failure.headline).toBe(`Preview build failed: ${LINE}`);
  expect(told("main").failure.headline).toBe(`Main build failed: ${LINE}`);
  expect(told(undefined).failure.headline).toBe(`Build failed: ${LINE}`);
});

test("the Session is told the build, the commit, the line and the log", () => {
  const { text } = told("preview");
  expect(text).toMatch(/code\s+bridge\.fleet\.build_failed/);
  expect(text).toMatch(/build\s+preview/);
  expect(text).toMatch(/commit\s+8944d2fca1b2c3d4e5f6/);
  expect(text).toMatch(/failed\s+error\[E0432\]: unresolved import crate::drones/);
  expect(text).toMatch(/log\s+~\/Library\/Logs\/Armada\/restart-build\.log/);
});

test("a build this window did not see start is not named in the fields", () => {
  expect(told(undefined).text).not.toMatch(/^ +build /m);
});
