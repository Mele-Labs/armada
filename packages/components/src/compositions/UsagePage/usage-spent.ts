// What the tokens were spent on, as the Usage page's Spent on view draws it. These types are the page's own, and Fleet's
// `get_usage_spent` carries the same splits under each Job's id. Units throughout, weighed by model and cache lifetime as the
// rest of the page is.

export const SPENT_ON = [
  "start_up",
  "reads",
  "own_words",
  "output",
  "cold_cache",
  "announced",
  "other_results",
  "messages",
  "background",
] as const;
export type SpentOn = (typeof SPENT_ON)[number];

export const SPENT_ON_LABEL: Readonly<Record<SpentOn, string>> = {
  start_up: "Opening text",
  reads: "Reads carried",
  own_words: "Own words carried",
  output: "Output",
  cold_cache: "Cold cache rewrites",
  announced: "Announced again",
  other_results: "Other tool results",
  messages: "Messages and skills",
  background: "Background summaries",
};

/** What each part means: the tooltip on its name. */
export const SPENT_ON_MEANS: Readonly<Record<SpentOn, string>> = {
  start_up: "The text a thread opens with, and the same text again after each compaction",
  reads: "Read results, searches, and grep, sed and cat through Bash, carried on every later turn",
  own_words: "The agent's own text, tool calls and thinking, carried on every later turn",
  output: "Tokens the model wrote",
  cold_cache: "The whole context written again after the cache went cold",
  announced: "Connector instructions, tool lists and reminders sent again partway through a thread",
  other_results: "Results of builds, git, the browser and Fleet calls, carried on every later turn",
  messages: "Messages to the agent, and the skill bodies it loaded",
  background: "Summaries written in the background",
};

export const SPENT_BY = [
  "main",
  "subagents",
  "background",
] as const;
export type SpentBy = (typeof SPENT_BY)[number];

export const SPENT_BY_LABEL: Readonly<Record<SpentBy, string>> = {
  main: "Main threads",
  subagents: "Subagents",
  background: "Background",
};

export const SPENT_BY_MEANS: Readonly<Record<SpentBy, string>> = {
  main: "The thread a Job's agent runs in",
  subagents: "Agents a main thread started",
  background: "Summaries and titles written outside any thread",
};

/** Units in each part, twice over: by what they went on, and by who spent them. Each adds up to the whole. */
export type SpentSplit = {
  on: Readonly<Record<SpentOn, number>>;
  by: Readonly<Record<SpentBy, number>>;
};

export type SpentJob = {
  id: string;
  title: string;
  split: SpentSplit;
};

/** One window: its whole, and the Jobs that spent in it, most first. */
export type SpentWindow = {
  id: string;
  label: string;
  total: SpentSplit;
  jobs: readonly SpentJob[];
};

/** The windows on offer, shortest first. */
export type UsageSpent = {
  windows: readonly SpentWindow[];
};

export const unitsOf = (split: SpentSplit): number =>
  SPENT_ON.reduce((sum, one) => sum + split.on[one], 0);
