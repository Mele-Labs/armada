// Autopilot's marks on the Bays metro, laid over what `metro.ts` drew and only while an Autopilot is
// on: its lines wear its glyph, and a pick whose Job has not started leaves Waiting for the faint Next
// up station before it, as a dashed stop. Kept out of `metro.ts` so the metro reads the same with none.
// #2092.

import type { Autopilot, AutopilotPick, JobSummary } from "@armada/protocol";

import { STATIONS, type Line, type Metro, type Stop } from "./metro";

/** The origin Fleet gives a Job Autopilot started. */
export const PICKED = "autopilot_picked";

function nextUpStop(pick: AutopilotPick, job: JobSummary): Stop {
  return {
    tone: "idle",
    card: {
      tone: "idle",
      pill: "picked",
      title: "Picked by Autopilot, not started",
      rows: [
        ["Issue", `#${pick.issue_number} ${pick.issue_title}`],
        ["Why", pick.reason],
      ],
      acts: [{ kind: "job", id: job.id, label: "Open Job" }],
    },
  };
}

/** The metro with every on Autopilot's marks on it. */
export function withAutopilot(metro: Metro, autopilots: readonly Autopilot[]): Metro {
  const picks = new Map<string, AutopilotPick>();
  for (const autopilot of autopilots) {
    if (autopilot.state === "stopped") continue;
    for (const pick of autopilot.picks) if (pick.job_id !== undefined && pick.state === "started") picks.set(pick.job_id, pick);
  }
  const theirs = (line: Line) => {
    const job = line.item?.job;
    return job !== undefined && (picks.has(job.id) || job.origin === PICKED);
  };
  const marked = (line: Line): Line => (theirs(line) ? { ...line, autopilot: true } : line);
  const waiting: Line[] = [];
  const nextUp: Line[] = [];
  for (const line of metro.waiting) {
    const job = line.item?.job;
    const pick = job === undefined ? undefined : picks.get(job.id);
    if (job === undefined || pick === undefined || job.started_at !== undefined) waiting.push(marked(line));
    else nextUp.push({ ...line, autopilot: true, reach: -1, stops: STATIONS.map(() => undefined), nextUp: nextUpStop(pick, job) });
  }
  // A pick moved to Next up no longer counts at Waiting.
  const counts = metro.counts.map((station, at) =>
    at !== 0
      ? station
      : station.flatMap(([tone, n]) => {
          const left = n - nextUp.filter((line) => (metro.waiting.find((one) => one.key === line.key)?.stops[0]?.tone ?? line.tone) === tone).length;
          return left > 0 ? [[tone, left] as const] : [];
        }),
  );
  return {
    ...metro,
    harbours: metro.harbours.map((harbour) => ({ ...harbour, lines: harbour.lines.map(marked) })),
    waiting,
    nextUp,
    counts,
  };
}
