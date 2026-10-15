// A twelve-hour Session built on the battlefield, in nine moments a walk steps through. Each moment is the `build` Fleet would carry on
// the Session's record at that point (`SessionBuild`, the wire's own shape): the phase, a piece for each task, the stops of the road
// with a forecast for those not reached, the subagents running and whether it has stalled. Minutes are from the Session's start;
// the moment sets how long ago each instant was, against the clock the window runs on.

import type { BuildStop, SessionBuild } from "@armada/protocol";

/** The Session the field is about. */
export const FEATURED = "bp1";

/** The moments, as minutes into the Session: scope, plan, implement, quiet ten minutes, quiet forty, the road begun, the road run, landed, twenty-six minutes on. */
const NOW = [50, 130, 342, 350, 380, 540, 575, 590, 610];
export const MOMENTS = NOW.length;

const AGENTS = ["Explore: map the dock call sites", "Plan: split the dock panel", "general-purpose: write the dock tests", "Explore: check the token use"];
const TASKS = ["Dock layout", "Dock store", "Panel header", "Drag handles", "Keyboard map", "Settings page", "Dock tests"];
const FORECAST: BuildStop["id"][] = ["walk", "pushed", "pr", "checks", "auto_merge"];

/** The tasks as each piece stands: ghosts, then four done with the fourth done without an edit seen, the next in scaffold; later all done. */
const PIECES: Record<number, SessionBuild["pieces"][number]["piece"][]> = {
  1: ["ghost", "ghost", "ghost", "ghost", "ghost", "ghost", "ghost"],
  2: ["solid", "solid", "solid", "hollow", "scaffold", "ghost", "ghost"],
  5: ["solid", "solid", "solid", "hollow", "solid", "hollow", "solid"],
};
const piecesAt = (moment: number) => PIECES[moment < 2 ? moment : moment < 5 ? 2 : 5] ?? [];

/** The road at a moment: what was reached by then, in order, and the rest as forecast. */
const REACHED: [BuildStop["id"], number][] = [["walk", 500], ["pushed", 510], ["pr", 520], ["checks", 555], ["auto_merge", 570]];

/** The `build` of the Session at a moment of the story. */
export function buildAt(moment: number): SessionBuild {
  const m = Math.min(moment, NOW.length - 1);
  const now = NOW[m]!;
  // Half a minute older than the minute it names, so a figure read a few seconds later still reads as that minute.
  const when = (minute: number) => new Date(Date.now() - (now - minute) * 60_000 - 30_000).toISOString();
  const reached = REACHED.filter(([, minute]) => minute <= now);
  const stops: BuildStop[] = [...reached.map(([id, minute]) => ({ id, at: when(minute) })), ...FORECAST.filter((id) => !reached.some(([one]) => one === id)).map((id) => ({ id }))];
  const stalled = m === 3 || m === 4;
  const pieces = piecesAt(m).map((piece, i) => ({ id: `t${i + 1}`, text: TASKS[i]!, piece }));
  return {
    phase: m >= 7 ? "landed" : m >= 5 ? "road" : m >= 2 ? "implement" : m === 1 ? "plan" : "scope",
    pieces,
    stops,
    forecast: FORECAST,
    ...(m >= 7 ? { landed_at: when(584) } : {}),
    ...(m >= 2 ? { quiet_since: when(m <= 4 ? 340 : now) } : {}),
    stalled,
    workers: m === 2 ? AGENTS : m === 3 || m === 4 ? [AGENTS[3]!] : [],
  };
}

/** A second Session's ring for the last moment of the walk: the Session's own tasks and two subagents' pieces, one subagent still running and one finished. */
export const SUBAGENTS = "bp3";
const RUNNING = "Explore: map the worker call sites";
const ENDED = "general-purpose: write the ring tests";

export function subagentsBuild(): SessionBuild {
  const piece = (id: string, text: string, kind: SessionBuild["pieces"][number]["piece"], by?: string) => ({ id, text, piece: kind, ...(by === undefined ? {} : { by }) });
  return {
    phase: "implement",
    pieces: [
      piece("s1", "Read the ring code", "solid"),
      piece("s2", "Find the worker call sites", "solid", RUNNING),
      piece("s3", "List the tooltips that name a worker", "scaffold", RUNNING),
      piece("s4", "Test where a worker sits", "solid", ENDED),
      piece("s5", "Test a piece's tooltip", "hollow", ENDED),
      piece("s6", "Draw the workers in the column", "ghost"),
    ],
    stops: [],
    forecast: [],
    quiet_since: new Date(Date.now() - 60_000).toISOString(),
    stalled: false,
    workers: [RUNNING],
  };
}
