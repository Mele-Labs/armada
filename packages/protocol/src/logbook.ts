/**
 * `recall`: `GET /logbook/recall?query=&section=&manifest_id=`. The Logbook entries whose text holds the
 * words asked for, best first, at most 20. Faded and candidate entries are in it.
 */
export type Recalled = {
  entries: RecalledEntry[];
};

export type RecalledEntry = {
  section: "repo" | "habits" | "machine";
  kind: "command" | "gotcha" | "convention" | "environment" | "habit";
  state: "candidate" | "standing" | "faded";
  text: string;
  /** How many distinct Sessions bore it out. */
  confirmed_by: number;
};

/** `get_logbook_for_subagent`'s body: `POST /logbook/subagent`. The mod's hook sends it. */
export type SubagentAsk = {
  session_id: string;
  prompt: string;
};

/** What a subagent is handed. Empty where the Logbook holds nothing for it. */
export type ForSubagent = {
  text: string;
};
