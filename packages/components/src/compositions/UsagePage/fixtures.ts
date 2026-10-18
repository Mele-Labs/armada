// A real 7-day baseline, 11 to 17 Oct, as the mock draws it. The plan windows, the day totals, the session rows and the
// four whole-week splits are measured. Each session's own split by model, tool and day is a sketch in the same shape.

export type Slice = { name: string; share: number; /** The commands a row is made of, biggest first. */ top?: string; /** All of them, for a tooltip. */ all?: string };

export type UsageSession = {
  id: string;
  title: string;
  /** Cost units, millions. */
  units: number;
  /** Share of the burn that ran in subagents, percent. */
  subagent: number;
  /** Peak context in thousands of tokens. */
  peak: number;
  model: string;
  /** Share of the burn above 200k context, percent. A sketch. */
  above200: number;
  byModel: Slice[];
  byTool: Slice[];
  /** The Bash commands, as a share of this Session's whole burn. */
  byBash: Slice[];
  /** Bash's own share of the burn, percent: what byBash is a part of. */
  bashShare: number;
  byToken: Slice[];
  /** Units per day, 11 to 17 Oct. */
  byDay: number[];
};

export const DAYS = ["Oct 11", "Oct 12", "Oct 13", "Oct 14", "Oct 15", "Oct 16", "Oct 17"] as const;

export const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] as const;
export const HOURS = ["00h", "03h", "06h", "09h", "12h", "15h", "18h", "21h"] as const;

/** Measured: units by 3 hour block (rows) and day (columns, Sun 11 to Sat 17). Sunday is partial. */
export const HEAT: number[][] = [
  [0.0, 50.6, 39.3, 37.2, 5.6, 0.2, 37.4],
  [0.0, 3.3, 0.0, 0.0, 0.0, 0.0, 52.7],
  [0.0, 2.4, 10.3, 15.5, 11.2, 5.5, 72.8],
  [0.0, 79.4, 69.9, 13.7, 29.4, 63.4, 58.6],
  [0.0, 16.6, 84.0, 18.5, 8.3, 54.7, 32.5],
  [0.0, 0.0, 23.6, 53.8, 52.5, 47.2, 14.5],
  [0.0, 0.0, 1.9, 38.7, 8.3, 26.4, 0.0],
  [69.9, 103.7, 51.5, 82.6, 65.6, 107.6, 34.9],
];

/** The headline tiles a week earlier. A sketch: only the week's own figures are measured. */
export const PRIOR_WEEK = { plan: 58, subagent: 66, above200: 61 };

export const PLAN_WINDOWS = [
  { id: "5h", label: "5 hour window", percent: 4, resets: undefined },
  { id: "7d", label: "7 day window", percent: 66, resets: "Oct 22" },
] as const;

export const BURN_PER_DAY = [69.9, 260.8, 285.1, 264.9, 184.2, 305.0, 292.3];

const spread = (units: number, weights: number[]): number[] => {
  const total = weights.reduce((a, b) => a + b, 0);
  return weights.map((w) => Math.round(((units * w) / total) * 10) / 10);
};


/** Measured, week: each turn is charged to the first tool it called, so a share is a turn's whole cost, mostly re-reading context. */
export const WEEK_TOOLS: Slice[] = [
  { name: "Bash", share: 79.0 }, { name: "No tool call", share: 7.6 }, { name: "Read", share: 3.5 }, { name: "Write", share: 2.1 },
  { name: "SendMessage", share: 1.3 }, { name: "Edit", share: 1.1 }, { name: "ToolSearch", share: 0.8 }, { name: "fleet waiting_for", share: 0.8 },
  { name: "Agent", share: 0.8 }, { name: "Monitor", share: 0.5 }, { name: "Skill", share: 0.4 }, { name: "fleet show_window", share: 0.4 },
  { name: "AskUserQuestion", share: 0.2 }, { name: "TaskUpdate", share: 0.2 },
];
/** Measured, week: Bash grouped by what it was for, as a share of the whole week. The rows add up to Bash's 79.0%. */
export const WEEK_BASH: Slice[] = [
  { name: "Read and search files", share: 42.2, top: "grep, sed, cat", all: "grep 15.1, sed 13.1, cat 6.0, ls 2.1, git grep 1.0, rg 1.0, wc 0.6, tail 0.3, git show, log and diff, and others" },
  { name: "Scripts, python3 and node", share: 14.9, top: "python3 stdin or -c, python3 -I", all: "python3 stdin or -c 13.0, python3 -I 1.3, node" },
  { name: "Git", share: 7.1, top: "status, add, fetch", all: "status 2.2, add 1.2, fetch 1.1, branch 0.8, push 0.3" },
  { name: "Build, test and checks", share: 4.7, top: "pnpm exec, taskpolicy, armada check", all: "pnpm exec 1.0, taskpolicy 1.0, armada check 0.7, npx vitest 0.2, cargo xtask 0.2" },
  { name: "Loops, waits and Fleet calls", share: 3.6, top: "for, armada worktree, sleep", all: "for 2.2, armada worktree 0.4, sleep, until, curl" },
  { name: "GitHub", share: 2.2, top: "gh pr, gh api, gh run", all: "gh pr 1.3, gh api 0.5, gh run 0.4" },
  { name: "Other", share: 4.3, top: "cp, mkdir, rm", all: "cp, mkdir, rm, scripts/preview, ps, perl, which, echo" },
];
/** Measured, week. Input is under 0.1%. */
export const WEEK_TOKENS: Slice[] = [
  { name: "cache read", share: 69.8 }, { name: "cache creation", share: 18.4 }, { name: "output", share: 11.8 }, { name: "input", share: 0.04 },
];

// A Session's own splits are a sketch: the week's, nudged by a fixed amount per Session so each reads differently.
const nudge = (rows: Slice[], seed: number): Slice[] => rows.map((one, i) => ({ name: one.name, share: one.share * (1 + (((seed * 7 + i * 5) % 9) - 4) / 10) }));
const sum = (rows: Slice[]) => rows.reduce((a, b) => a + b.share, 0);
const to100 = (rows: Slice[], places: number): Slice[] => rows.map((one) => ({ ...one, share: Number(((one.share * 100) / sum(rows)).toFixed(places)) }));
const toolsFor = (seed: number) => to100(nudge(WEEK_TOOLS, seed), 1);
const bashShareFor = (seed: number) => toolsFor(seed).find((one) => one.name === "Bash")!.share;
const bashFor = (seed: number): Slice[] => {
  const rows = nudge(WEEK_BASH, seed + 3);
  const scale = bashShareFor(seed) / sum(rows);
  return rows.map((one) => ({ ...one, share: Number((one.share * scale).toFixed(1)) }));
};
const tokensFor = (seed: number) => to100(nudge(WEEK_TOKENS, seed + 5), 2);

export const SESSIONS: UsageSession[] = [
  {
    id: "merger", above200: 61, title: "Jobs -> Session Merger", units: 344.6, subagent: 87, peak: 372, model: "opus+sonnet",
    byModel: [{ name: "sonnet", share: 81.2 }, { name: "opus", share: 18.6 }, { name: "haiku", share: 0.2 }],
    byTool: toolsFor(1), byBash: bashFor(1), bashShare: bashShareFor(1), byToken: tokensFor(1),
    byDay: spread(344.6, [0, 10, 24, 30, 12, 16, 8]),
  },
  {
    id: "cockpit", above200: 58, title: "Cockpit Updates", units: 166.0, subagent: 85, peak: 368, model: "opus+sonnet",
    byModel: [{ name: "sonnet", share: 74.5 }, { name: "opus", share: 25.4 }, { name: "haiku", share: 0.1 }],
    byTool: toolsFor(2), byBash: bashFor(2), bashShare: bashShareFor(2), byToken: tokensFor(2),
    byDay: spread(166, [6, 18, 14, 10, 6, 28, 18]),
  },
  {
    id: "debug", above200: 79, title: "Debug Job 14", units: 116.7, subagent: 66, peak: 962, model: "opus",
    byModel: [{ name: "opus", share: 71.0 }, { name: "sonnet", share: 28.8 }, { name: "haiku", share: 0.2 }],
    byTool: toolsFor(3), byBash: bashFor(3), bashShare: bashShareFor(3), byToken: tokensFor(3),
    byDay: spread(116.7, [0, 0, 8, 30, 22, 12, 0]),
  },
  {
    id: "merge-line", above200: 47, title: "Multi Repo Merge Line and PR Monitoring", units: 72.4, subagent: 77, peak: 367, model: "sonnet",
    byModel: [{ name: "sonnet", share: 93.0 }, { name: "opus", share: 6.9 }, { name: "haiku", share: 0.1 }],
    byTool: toolsFor(4), byBash: bashFor(4), bashShare: bashShareFor(4), byToken: tokensFor(4),
    byDay: spread(72.4, [4, 12, 10, 8, 14, 10, 6]),
  },
  {
    id: "syncing", above200: 49, title: "Syncing Armada between Machines", units: 70.2, subagent: 73, peak: 368, model: "sonnet",
    byModel: [{ name: "sonnet", share: 88.4 }, { name: "opus", share: 11.5 }, { name: "haiku", share: 0.1 }],
    byTool: toolsFor(5), byBash: bashFor(5), bashShare: bashShareFor(5), byToken: tokensFor(5),
    byDay: spread(70.2, [0, 2, 6, 14, 16, 12, 10]),
  },
  {
    id: "helm", above200: 66, title: "Helm as a distributor", units: 68.9, subagent: 71, peak: 616, model: "opus+sonnet",
    byModel: [{ name: "opus", share: 52.3 }, { name: "sonnet", share: 47.5 }, { name: "haiku", share: 0.2 }],
    byTool: toolsFor(6), byBash: bashFor(6), bashShare: bashShareFor(6), byToken: tokensFor(6),
    byDay: spread(68.9, [0, 0, 0, 6, 12, 22, 20]),
  },
];

/** The whole week, every session. */
export const WEEK = {
  byModel: [{ name: "sonnet", share: 76.3 }, { name: "opus", share: 23.5 }, { name: "haiku", share: 0.2 }] as Slice[],
  byTool: WEEK_TOOLS,
  byBash: WEEK_BASH,
  bashShare: 79.0,
  byToken: WEEK_TOKENS,
  mainShare: 30,
  subagentShare: 70,
  above200: 56,
  contextAbove: [{ name: "Above 200k", share: 56.4 }, { name: "Above 300k", share: 26.1 }] as Slice[],
};
