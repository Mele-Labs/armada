// Copy debug report: the text a person pastes to whoever helps them. `GET /debug_report?root=`.
// Hand-mirrored from `crates/ipc/src/debug_report.rs`.
//
// The header rules in `protocol.ts` hold here.

/**
 * `get_debug_report`'s answer: one string laid out as Install, Mod, This repository (where `root` named
 * one), a block for each live Job, Recent trouble, and a last line `fleet protocol <id>  taken <time>`.
 * Names, versions and states only, with the home folder written as `~`. Nothing is uploaded.
 */
export type DebugReport = {
  text: string;
};
