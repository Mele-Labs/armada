// Teaching: what a person says about a thing they pressed Teach on, read into a proposal, kept, and the words a
// kept teaching opens. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
// `docs/concepts/mods.md`.

import type { ReadTeaching, SaveTeaching, TeachingRead, TeachingSaved, WordRulesRead } from "@armada/protocol";

export type TeachingApi = {
  /** `POST /teachings/read`: one model call, nothing kept. The answer has at least one part. */
  readTeaching: (read: ReadTeaching) => Promise<TeachingRead>;
  /** `POST /teachings`: the proposal as the person left it. Checked whole before anything is written. */
  saveTeaching: (save: SaveTeaching) => Promise<TeachingSaved>;
  /** `GET /teachings/words`: the patterns with an address for `repository`, or for every repository where none is named. */
  listWordRules: (repository?: string, sessionId?: string) => Promise<WordRulesRead>;
};

export const TEACHING_CHANNELS = {
  readTeaching: "bridge:read-teaching",
  saveTeaching: "bridge:save-teaching",
  listWordRules: "bridge:list-word-rules",
} as const;
