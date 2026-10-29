// What a Job costs the machine, for its System layer — `crates/ipc/src/session_system.rs`.
//
// Read off `get_session_system` (`GET /sessions/system?session_id=`) and acted on with `stop_process`
// (`POST /sessions/stop_process`). The header rules in `protocol.ts` hold here: hand-written, and the
// sets Fleet closes are written out.

/** Whether a process still runs. Fleet answers `running`; Bridge keeps one it has just stopped as `stopped` until the next reading. */
export type SystemProcessState = "running" | "stopped";

/** One process the Job started and that still runs. */
export type SystemProcess = {
  pid: number;
  /** The command line, as `ps` shows it. */
  command: string;
  /** The TCP port it listens on, where it does. */
  port?: number;
  state: SystemProcessState;
};

/** A worktree the Job holds. */
export type SystemWorktree = {
  slot: number;
  path: string;
  /** The branch it is on, where one is recorded. */
  branch?: string;
};

/** A file Fleet keeps for the Job: `session.log`, or the agent's transcript. */
export type SystemLog = { name: string; bytes: number };

/** `get_session_system`. A figure that could not be read is absent, never zero. */
export type SessionSystem = {
  session_id: string;
  processes: SystemProcess[];
  /** The processes' CPU together, as a percentage of one core. */
  cpu_percent?: number;
  /** The processes' resident memory together. */
  memory_bytes?: number;
  /** The machine's memory. */
  memory_of_bytes?: number;
  /** What the Job's slot takes on disk. */
  disk_bytes?: number;
  /** The size of the volume the slot is on. */
  disk_of_bytes?: number;
  worktrees: SystemWorktree[];
  logs: SystemLog[];
};

/** `stop_process`: exactly one of `pid` and `port`. `session_id` is read only where the connection places no hosted Session. */
export type StopProcess = { session_id?: string; pid?: number; port?: number };

/** What `stop_process` answers: the process that was ended. */
export type ProcessStopped = { pid: number; command: string };
