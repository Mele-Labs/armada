// The text a Session is told when a failure toast hands it over: the debug info Copy would carry, then the window as it
// stood at the press, written with the lines an annotation on the whole window uses. Pure, so the press reads the page and
// Bridge's state once and this only writes them.

import { debugInfo, type DebugPayload } from "@armada/components";
import type { FleetBuildReport } from "@armada/protocol";

import { sessionFacts, sessionLine, windowLines, type WindowFacts } from "../../shared/annotations";
import type { BridgeState } from "../../shared/bridge";

/** What the message reads of Bridge's state beyond the window's facts: the Sessions, and the build Fleet reported. */
export type PressedState = Pick<BridgeState, "sessions" | "fleetBuild">;

/** Named by a path nothing writes yet (`failures/notice.ts`), and a renderer cannot check a path exists, so a Session is not pointed at it. */
const UNWRITTEN = "bridge_log";

/** The build Fleet runs on, and where it stands against `origin/main`. */
function buildLine(build: FleetBuildReport): string {
  const facts = [
    build.commit === undefined ? build.on : `${build.on} ${build.commit.slice(0, 9)}`,
    ...(build.position === undefined ? [] : [`${build.position.ahead} ahead of origin/main`, `${build.position.behind} behind`]),
    ...(build.restarting === undefined ? [] : [`restarting onto ${build.restarting}`]),
    ...(build.failed === undefined ? [] : [`last restart failed: ${build.failed}`]),
  ];
  return `- Fleet build: ${facts.join(", ")}`;
}

/** The Session the refused act was for, where it is not the one open: the open one is already in the window's lines. */
function actSessionLine(payload: DebugPayload, view: WindowFacts, state: PressedState): string | null {
  const id = payload.fields?.find((field) => field.key === "session_id")?.value;
  if (id === undefined || id === view.context?.session?.id || state.sessions.state !== "read") return null;
  const session = state.sessions.sessions.find((one) => one.id === id);
  return session === undefined ? `- Act's Job: ${id}, not in Fleet's list` : sessionLine("Act's Job", sessionFacts(session));
}

/** The message that carries a failure to a new Session. */
export function failureMessage(payload: DebugPayload, view: WindowFacts, state: PressedState): string {
  const info = debugInfo({ ...payload, fields: (payload.fields ?? []).filter((field) => field.key !== UNWRITTEN) });
  const lines = windowLines(view);
  const acted = actSessionLine(payload, view, state);
  if (acted !== null) lines.push(acted);
  if (state.fleetBuild !== null) lines.push(buildLine(state.fleetBuild));
  return `${info}\n\nbridge window\n${lines.join("\n")}`;
}
