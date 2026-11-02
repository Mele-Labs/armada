// The flight deck with something to read in the Timeline HUD: the ship's stuck Jobs, and over the last two hours a pull request
// merged, Jobs finished, and main going red and green again. A pull request comes through to a row by the Job that holds it, here
// Migration notes, which holds #1892. The walk `timeline-hud` plays it.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as base } from "./cockpit-ship";

const FORGE = "https://git.example/armada/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const held = base.draft?.events ?? [];

let count = 7000;
function entry(
  minutes: number,
  subject: TimelineSubject,
  title: string | undefined,
  kind: TimelineEvent["kind"],
  tone: TimelineEvent["tone"],
  says: string,
  detail?: string,
): TimelineEvent {
  count += 1;
  return {
    id: count,
    at: ago(minutes),
    kind,
    subject,
    tone,
    says,
    manifest_id: "armada",
    ...(title === undefined
      ? {}
      : {
          title,
        }),
    ...(detail === undefined
      ? {}
      : {
          detail,
        }),
  };
}

const MAIN: TimelineSubject = {
  kind: "main",
};
const MORE: TimelineEvent[] = [
  entry(
    18,
    {
      kind: "pull_request",
      number: 1892,
      url: `${FORGE}1892`,
    },
    "Add the pause marker column",
    "pr.merged",
    "ok",
    "Merged",
    "fleet/pause-marker-column",
  ),
  // Gone from the ship, so its row opens nothing.
  entry(
    47,
    {
      kind: "session",
      id: "01SESSIONRELEASENOTES0001",
    },
    "Release notes",
    "session.ended",
    "ok",
    "Done",
  ),
  entry(61, MAIN, undefined, "main.red", "bad", "Main red", "components_test"),
  entry(38, MAIN, undefined, "main.green", "ok", "Main green"),
];

export const s253TimelineHud: Scenario = {
  ...base,
  name: "timeline-hud",
  says: "The flight deck with stuck Jobs, and a merge, two finished Jobs and main red then green in the last two hours, for the Timeline HUD",
  draft: {
    ...base.draft,
    events: [
      ...held,
      ...MORE,
    ],
  },
};
