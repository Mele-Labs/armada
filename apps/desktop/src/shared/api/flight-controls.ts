// The Cockpit's flight controls, kept in Kit, and the shell line one of them runs. A slice imports protocol, never another slice;
// `../api.ts` and `../bridge.ts` compose them.

import type { FlightControls, FlightControlsRead, Outcome, SessionShellRan } from "@armada/protocol";

/** What reading or saving the flight controls came to: Fleet's reading of the file, or the refusal to word. */
export type FlightControlsAnswer = { ok: true; value: FlightControlsRead } | { ok: false; outcome: Outcome };

/** What running a shell line came to: how it ended, or the refusal to word. */
export type SessionShellAnswer = { ok: true; value: SessionShellRan } | { ok: false; outcome: Outcome };

export type FlightControlsApi = {
  /** `GET /kit/flight_controls`: the file as Fleet reads it, or why it could not. */
  getFlightControls: () => Promise<FlightControlsAnswer>;
  /** `POST /kit/flight_controls`: the whole file, answered with Fleet's reading of it again. */
  saveFlightControls: (controls: FlightControls) => Promise<FlightControlsAnswer>;
  /** `POST /sessions/shell`: a line run in the Job's slot, answered once it has ended. */
  runSessionShell: (sessionId: string, command: string) => Promise<SessionShellAnswer>;
};

export const FLIGHT_CONTROLS_CHANNELS = {
  getFlightControls: "bridge:get-flight-controls",
  saveFlightControls: "bridge:save-flight-controls",
  runSessionShell: "bridge:run-session-shell",
} as const;
