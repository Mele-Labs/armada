// A piloted Job's row and the note a person leaves on an exit.
// `crates/ipc/src/piloting.rs`, `docs/concepts/pilot.md`.
//
// The header rules in `protocol.ts` hold here: hand-written, and every closed
// set is left as `string`.

/** The body of `attest_complete` and `close_as_superseded`. Never required. */
export type PilotNote = {
  note?: string;
};

/**
 * A job's pilot on its row, **beside the status and never instead of it**.
 * Present on a `piloted` job and still present once the pilot ended, because
 * `exit` is what tells a job a person attested from one that passed its gates.
 * `JobSummary.piloted`.
 */
export type Piloted = {
  /** `take_over` or `restart_step`. */
  reason: string;
  since: string;
  /**
   * `submitted`, `attested` or `superseded`. Absent while the person is still
   * working. **`attested` is never verified**, and is drawn as what it is.
   */
  exit?: string;
  ended_at?: string;
  /** The person's words on an attestation or a supersede. */
  note?: string;
};
