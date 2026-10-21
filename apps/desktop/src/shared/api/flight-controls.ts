// The Cockpit's flight controls, kept in Kit, and the shell line one of them runs. A slice imports protocol, never another slice;
// `../api.ts` and `../bridge.ts` compose them.

import type { FlightControls, FlightControlsRead, Outcome, RepositoryFlightControls, SessionShellRan } from "@armada/protocol";
import type { ApproveRepositoryTile, RepositoryTile } from "@armada/protocol";

/** What reading or saving the flight controls came to: Fleet's reading of the file, or the refusal to word. */
export type FlightControlsAnswer = { ok: true; value: FlightControlsRead } | { ok: false; outcome: Outcome };

/** What reading or saving one repository's flight controls came to: where Fleet read them and what it read, or the refusal to word. */
export type RepositoryFlightControlsAnswer = { ok: true; value: RepositoryFlightControls } | { ok: false; outcome: Outcome };

/** What running a shell line came to: how it ended, or the refusal to word. */
export type SessionShellAnswer = { ok: true; value: SessionShellRan } | { ok: false; outcome: Outcome };

export type FlightControlsApi = {
  /** `GET /kit/flight_controls`: the file as Fleet reads it, or why it could not. */
  getFlightControls: () => Promise<FlightControlsAnswer>;
  /** `POST /kit/flight_controls`: the whole file, answered with Fleet's reading of it again. */
  saveFlightControls: (controls: FlightControls) => Promise<FlightControlsAnswer>;
  /** `GET /repositories/flight_controls`: the actions kept for one repository, by its root, and those it holds as committed in the Job `sessionId` names. */
  getRepositoryFlightControls: (repository: string, sessionId?: string) => Promise<RepositoryFlightControlsAnswer>;
  /** `POST /repositories/flight_controls`: the repository's whole file, answered with Fleet's reading of it again. */
  saveRepositoryFlightControls: (repository: string, controls: FlightControls) => Promise<RepositoryFlightControlsAnswer>;
  /** `POST /sessions/shell`: a line run in the Job's slot, answered once it has ended. A tile the repository committed names itself in `tile`, and Fleet runs the command it holds. */
  runSessionShell: (sessionId: string, command: string, tile?: RepositoryTile) => Promise<SessionShellAnswer>;
  /** `POST /repositories/flight_controls/approve`: this Mac may run the command shown, by its hash. Answers the repository's read again. */
  approveRepositoryTile: (body: ApproveRepositoryTile) => Promise<RepositoryFlightControlsAnswer>;
};

export const FLIGHT_CONTROLS_CHANNELS = {
  getFlightControls: "bridge:get-flight-controls",
  saveFlightControls: "bridge:save-flight-controls",
  getRepositoryFlightControls: "bridge:get-repository-flight-controls",
  saveRepositoryFlightControls: "bridge:save-repository-flight-controls",
  runSessionShell: "bridge:run-session-shell",
  approveRepositoryTile: "bridge:approve-repository-tile",
} as const;
