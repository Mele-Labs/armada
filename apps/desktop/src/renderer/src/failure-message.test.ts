// What a Session is told when a failure toast hands it over: the debug info, then the window at the press.
// The `bridge.not_sent` case is the real one that took a dig through the store to explain.

import { expect, test } from "vitest";
import type { DebugPayload } from "@armada/components";
import type { BridgeIdentity, FleetBuildReport } from "@armada/protocol";
import { refusedWith } from "@armada/protocol";
import { raisedFailure } from "./failing";
import { failureMessage, type PressedState } from "./failure-message";
import { actFields } from "./raised";
import type { WindowFacts } from "../../shared/annotations";

const BRIDGE: BridgeIdentity = { auditPath: "/Users/o/Library/Application Support/Armada/audit.jsonl", fleetProtocol: "32a42be6" };
const AT = "2026-10-17T00:29:54.243Z";

const OPEN = { id: "01OPEN", title: "Fix the flaky store test", state: "live", harness: "agent", cwd: "/repo" };
const OTHER = { id: "01OTHER", title: "Ask about folders", state: "live", harness: "agent", cwd: "/work", origin: "bridge" };

const VIEW: WindowFacts = {
  screen: "Sessions",
  layer: null,
  location: "/index.html#sessions",
  scenario: null,
  window: { width: 1280, height: 800 },
  context: {
    connection: "connected",
    doctor: { summary: "1 warning", rows: [{ module: "SQLite", result: "warn", detail: "3 rows will not load" }] },
    session: OPEN,
  },
};

const BUILD: FleetBuildReport = { on: "preview", commit: "8944d2fca1b2c3d4e5f6", position: { ahead: 2, behind: 0 } };

function state(over: Partial<PressedState> = {}): PressedState {
  return { sessions: { state: "read", sessions: [{ ...OPEN, origin: "bridge" }, OTHER] as never }, fleetBuild: BUILD, ...over };
}

/** The payload a told answer to a card makes, as the toast builds it. */
function refusedAnswer(session: string): DebugPayload {
  const message = "an always answer names the folder it is for: the path asked about or one above it";
  const error = { code: "bridge.not_sent", message, run_id: "", chain: [], fields: actFields({
    act: "answer_session_ask",
    request: { session_id: session, call: "call-7", answer: "always_allow", folder: undefined },
    card: { ask: "Read /elsewhere/notes.md", ask_reaches: "/elsewhere/notes.md", ask_reaches_to: "read", ask_offers: "allow_once,always_allow" },
  }) };
  const failure = raisedFailure({ key: 0, outcome: { ok: false, why: "refused", error } }, BRIDGE);
  if (failure === null) throw new Error("a refusal is a failure");
  return { ...failure.payload, at: AT };
}

test("a refused act carries its name, its request and what the card asked", () => {
  const text = failureMessage(refusedAnswer(OPEN.id), VIEW, state());
  expect(text).toMatch(/act\s+answer_session_ask/);
  expect(text).toMatch(/call\s+call-7/);
  expect(text).toMatch(/answer\s+always_allow/);
  expect(text).toMatch(/ask_reaches\s+\/elsewhere\/notes.md/);
  // An unset part of the request is not a field.
  expect(text).not.toMatch(/^ +folder /m);
});

test("the window follows the debug info, written as an annotation on the window writes it", () => {
  const text = failureMessage(refusedAnswer(OPEN.id), VIEW, state());
  expect(text.indexOf("armada error")).toBe(0);
  expect(text.indexOf("bridge window")).toBeGreaterThan(text.indexOf("taken 2026-10-17T00:29:54.243Z"));
  expect(text).toContain("- Screen: Sessions");
  expect(text).toContain("- Location: /index.html#sessions");
  expect(text).toContain("- Window: 1280×800");
  expect(text).toContain('- Job open: "Fix the flaky store test" (01OPEN)');
  expect(text).toContain("- Connection to Fleet: connected");
  expect(text).toContain("  - SQLite: warn — 3 rows will not load");
});

test("a Session that is not the open one is named, and the open one is not named twice", () => {
  const elsewhere = failureMessage(refusedAnswer(OTHER.id), VIEW, state());
  expect(elsewhere).toContain('- Act\'s Job: "Ask about folders" (01OTHER) — state live, harness agent, cwd /work');
  const here = failureMessage(refusedAnswer(OPEN.id), VIEW, state());
  expect(here).not.toContain("Act's Job");
});

test("a Session Fleet does not list is still named by its id", () => {
  expect(failureMessage(refusedAnswer("01GONE"), VIEW, state())).toContain("- Act's Job: 01GONE, not in Fleet's list");
  expect(failureMessage(refusedAnswer("01GONE"), VIEW, state({ sessions: { state: "none" } }))).not.toContain("Act's Job");
});

test("the build says which tree and where it stands, and is left out where Fleet reported none", () => {
  expect(failureMessage(refusedAnswer(OPEN.id), VIEW, state())).toContain("- Fleet build: preview 8944d2fca, 2 ahead of origin/main, 0 behind");
  const restarting = state({ fleetBuild: { on: "main", restarting: "preview", failed: "merge conflict" } });
  expect(failureMessage(refusedAnswer(OPEN.id), VIEW, restarting)).toContain("- Fleet build: main, restarting onto preview, last restart failed: merge conflict");
  expect(failureMessage(refusedAnswer(OPEN.id), VIEW, state({ fleetBuild: null }))).not.toContain("Fleet build");
});

test("the log path nothing writes is not named", () => {
  const payload = refusedAnswer(OPEN.id);
  expect(payload.fields?.some((field) => field.key === "bridge_log")).toBe(true);
  expect(failureMessage(payload, VIEW, state())).not.toContain("audit.jsonl");
});

test("a failure with no act and no context is the debug info and the page", () => {
  const failure = raisedFailure({ key: 0, outcome: refusedWith(500, "", { method: "POST", path: "/x" }) as never }, BRIDGE);
  const bare: WindowFacts = { screen: null, layer: "Settings", location: "/", scenario: "empty", window: { width: 900, height: 600 } };
  const text = failureMessage({ ...(failure?.payload ?? { message: "m", bridgeProtocol: "00000000" }), at: AT }, bare, state({ fleetBuild: null }));
  expect(text).toContain("- Screen: none marked");
  expect(text).toContain("- Inside: Settings");
  expect(text).toContain("- Mock scenario: empty");
  expect(text).not.toContain("Session");
});
