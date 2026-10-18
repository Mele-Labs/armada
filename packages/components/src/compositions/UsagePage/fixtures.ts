// A real 7-day baseline, 11 to 17 Oct, as the mock draws it. The plan windows, the day totals, the session rows and the
// four whole-week splits are measured. Each session's own split by model, tool and day is a sketch in the same shape.

export type Slice = { name: string; share: number };

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
  byModel: Slice[];
  byTool: Slice[];
  /** Units per day, 11 to 17 Oct. */
  byDay: number[];
};

export const DAYS = ["Oct 11", "Oct 12", "Oct 13", "Oct 14", "Oct 15", "Oct 16", "Oct 17"] as const;

export const PLAN_WINDOWS = [
  { id: "5h", label: "5 hour window", percent: 4, resets: undefined },
  { id: "7d", label: "7 day window", percent: 66, resets: "Oct 22" },
] as const;

export const BURN_PER_DAY = [69.9, 260.8, 285.1, 264.9, 184.2, 305.0, 292.3];

const spread = (units: number, weights: number[]): number[] => {
  const total = weights.reduce((a, b) => a + b, 0);
  return weights.map((w) => Math.round(((units * w) / total) * 10) / 10);
};

export const SESSIONS: UsageSession[] = [
  {
    id: "merger", title: "Jobs -> Session Merger", units: 344.6, subagent: 87, peak: 372, model: "opus+sonnet",
    byModel: [{ name: "sonnet", share: 81.2 }, { name: "opus", share: 18.6 }, { name: "haiku", share: 0.2 }],
    byTool: [{ name: "Read", share: 3.9 }, { name: "Write", share: 2.4 }, { name: "SendMessage", share: 1.6 }, { name: "Edit", share: 1.2 }, { name: "ToolSearch", share: 0.8 }, { name: "Agent", share: 0.7 }],
    byDay: spread(344.6, [0, 10, 24, 30, 12, 16, 8]),
  },
  {
    id: "cockpit", title: "Cockpit Updates", units: 166.0, subagent: 85, peak: 368, model: "opus+sonnet",
    byModel: [{ name: "sonnet", share: 74.5 }, { name: "opus", share: 25.4 }, { name: "haiku", share: 0.1 }],
    byTool: [{ name: "Read", share: 3.2 }, { name: "Write", share: 1.8 }, { name: "Edit", share: 1.4 }, { name: "SendMessage", share: 1.1 }, { name: "Agent", share: 1.0 }, { name: "ToolSearch", share: 0.9 }],
    byDay: spread(166, [6, 18, 14, 10, 6, 28, 18]),
  },
  {
    id: "debug", title: "Debug Job 14", units: 116.7, subagent: 66, peak: 962, model: "opus",
    byModel: [{ name: "opus", share: 71.0 }, { name: "sonnet", share: 28.8 }, { name: "haiku", share: 0.2 }],
    byTool: [{ name: "Read", share: 4.8 }, { name: "Edit", share: 1.6 }, { name: "Write", share: 1.3 }, { name: "Agent", share: 0.9 }, { name: "ToolSearch", share: 0.8 }, { name: "SendMessage", share: 0.5 }],
    byDay: spread(116.7, [0, 0, 8, 30, 22, 12, 0]),
  },
  {
    id: "merge-line", title: "Multi Repo Merge Line and PR Monitoring", units: 72.4, subagent: 77, peak: 367, model: "sonnet",
    byModel: [{ name: "sonnet", share: 93.0 }, { name: "opus", share: 6.9 }, { name: "haiku", share: 0.1 }],
    byTool: [{ name: "Read", share: 3.0 }, { name: "SendMessage", share: 2.1 }, { name: "Write", share: 1.7 }, { name: "Edit", share: 1.0 }, { name: "Agent", share: 0.8 }, { name: "ToolSearch", share: 0.7 }],
    byDay: spread(72.4, [4, 12, 10, 8, 14, 10, 6]),
  },
  {
    id: "syncing", title: "Syncing Armada between Machines", units: 70.2, subagent: 73, peak: 368, model: "sonnet",
    byModel: [{ name: "sonnet", share: 88.4 }, { name: "opus", share: 11.5 }, { name: "haiku", share: 0.1 }],
    byTool: [{ name: "Read", share: 3.6 }, { name: "Write", share: 2.2 }, { name: "Edit", share: 1.2 }, { name: "SendMessage", share: 0.9 }, { name: "ToolSearch", share: 0.9 }, { name: "Agent", share: 0.6 }],
    byDay: spread(70.2, [0, 2, 6, 14, 16, 12, 10]),
  },
  {
    id: "helm", title: "Helm as a distributor", units: 68.9, subagent: 71, peak: 616, model: "opus+sonnet",
    byModel: [{ name: "opus", share: 52.3 }, { name: "sonnet", share: 47.5 }, { name: "haiku", share: 0.2 }],
    byTool: [{ name: "Read", share: 3.3 }, { name: "SendMessage", share: 2.6 }, { name: "Write", share: 1.9 }, { name: "Agent", share: 1.2 }, { name: "Edit", share: 0.9 }, { name: "ToolSearch", share: 0.8 }],
    byDay: spread(68.9, [0, 0, 0, 6, 12, 22, 20]),
  },
];

/** The whole week, every session. */
export const WEEK = {
  byModel: [{ name: "sonnet", share: 76.3 }, { name: "opus", share: 23.5 }, { name: "haiku", share: 0.2 }] as Slice[],
  byTool: [
    { name: "Read", share: 3.5 }, { name: "Write", share: 2.0 }, { name: "SendMessage", share: 1.4 },
    { name: "Edit", share: 1.1 }, { name: "ToolSearch", share: 0.9 }, { name: "Agent", share: 0.8 },
  ] as Slice[],
  mainShare: 30,
  subagentShare: 70,
  contextAbove: [{ name: "Above 200k", share: 56.4 }, { name: "Above 300k", share: 26.1 }] as Slice[],
};
