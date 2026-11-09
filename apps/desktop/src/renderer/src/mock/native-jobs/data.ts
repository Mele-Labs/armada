// Fixtures for the native Jobs screens mock, shaped like the Fleet views the native app reads:
// a list row is a `SessionRecord` (crates/ipc/src/sessions.rs) with its `in_flight`, and a thread
// is `get_job_thread`'s `JobThread` (crates/ipc/src/job_thread.rs). Field names follow the wire.

export type Turn = "working" | "idle" | "paused" | "stopped";
export type Phase = "read" | "write" | "run";

export type InFlight = {
  lifecycle: "pull_request" | "ticket" | "linked_job";
  key: string;
  label: string;
  state: string;
};

export type Waiting = { id: string; text: string; kind: "plan" | "walk" | "question" | "draft" | "run" };

export type Stretch = {
  title: string;
  state: "done" | "now" | "next";
  gate: "plan_approved" | "walk_approved" | "checks_green" | "merged";
};

export type Job = {
  id: string;
  title: string;
  manifest_id: string;
  turn: Turn;
  phase?: Phase;
  last_turn_at: number; // minutes ago
  slot?: number;
  branch: string;
  waiting_for: Waiting[];
  in_flight: InFlight[];
  stretches?: Stretch[];
  handed_from?: string;
  forked_from?: string;
  ended?: boolean;
  open_call?: { text: string; since: number };
};

export const JOBS: Job[] = [
  {
    id: "20b4e00a",
    title: "Plan and orchestrate the native rebuild",
    manifest_id: "armada",
    turn: "idle",
    last_turn_at: 1,
    slot: 3,
    branch: "armada/session-20b4e00a",
    waiting_for: [{ id: "plan", text: "Approve the plan for the Jobs screens", kind: "plan" }],
    in_flight: [
      { lifecycle: "linked_job", key: "34aa1317", label: "Native Bridge Foundation", state: "working" },
      { lifecycle: "linked_job", key: "ad34a1b8", label: "Jobs screens", state: "working" },
      { lifecycle: "pull_request", key: "3370", label: "In-flight list on a Job's record", state: "queued" },
    ],
    stretches: [
      { title: "Decisions", state: "done", gate: "merged" },
      { title: "Foundation", state: "now", gate: "walk_approved" },
      { title: "Fleet views", state: "done", gate: "merged" },
      { title: "Jobs screens", state: "now", gate: "walk_approved" },
      { title: "Dispatch and title bar", state: "next", gate: "walk_approved" },
      { title: "Job panes", state: "next", gate: "walk_approved" },
    ],
  },
  {
    id: "34aa1317",
    title: "Native Bridge Foundation",
    manifest_id: "armada",
    turn: "working",
    phase: "run",
    last_turn_at: 0,
    slot: 7,
    branch: "armada/session-34aa1317",
    waiting_for: [],
    in_flight: [],
    handed_from: "Plan and orchestrate the native rebuild",
    open_call: { text: "armada check test native_walk thread_keeping", since: 48 },
  },
  {
    id: "9ffc10e6",
    title: "Bridge shows the plan as progress",
    manifest_id: "armada",
    turn: "idle",
    last_turn_at: 6,
    slot: 5,
    branch: "armada/session-9ffc10e6",
    waiting_for: [{ id: "walk", text: "Walk the progress line takes", kind: "walk" }],
    in_flight: [],
  },
  {
    id: "699aab92",
    title: "Operator never starts Rest-of Jobs",
    manifest_id: "armada",
    turn: "working",
    phase: "write",
    last_turn_at: 0,
    slot: 2,
    branch: "armada/session-699aab92",
    waiting_for: [],
    in_flight: [],
  },
  {
    id: "cf2b4b49",
    title: "Workflow Evaluation",
    manifest_id: "armada",
    turn: "idle",
    last_turn_at: 4,
    slot: 6,
    branch: "armada/session-cf2b4b49",
    waiting_for: [],
    in_flight: [{ lifecycle: "pull_request", key: "3368", label: "Ship the workflows as gated plan groups", state: "checks" }],
  },
  {
    id: "edd39abf",
    title: "Fleet takes Bridge views",
    manifest_id: "armada",
    turn: "idle",
    last_turn_at: 9,
    branch: "armada/session-edd39abf",
    waiting_for: [],
    in_flight: [],
    handed_from: "Fleet takes Bridge views",
  },
  {
    id: "9d01d0d2",
    title: "Subagent Plan Group",
    manifest_id: "armada",
    turn: "stopped",
    last_turn_at: 22,
    slot: 1,
    branch: "armada/session-9d01d0d2",
    waiting_for: [],
    in_flight: [],
  },
  {
    id: "6f425257",
    title: "Operator panel in Bridge",
    manifest_id: "armada",
    turn: "paused",
    last_turn_at: 4300,
    branch: "armada/session-6f425257",
    waiting_for: [],
    in_flight: [{ lifecycle: "pull_request", key: "3349", label: "Give the Operator three levels", state: "open" }],
  },
  {
    id: "8fd49e7c",
    title: "Temporary Operator",
    manifest_id: "armada",
    turn: "idle",
    last_turn_at: 2,
    slot: 4,
    branch: "armada/session-8fd49e7c",
    waiting_for: [],
    in_flight: [],
  },
  {
    id: "b31c0a77",
    title: "Pricing page copy",
    manifest_id: "website",
    turn: "idle",
    last_turn_at: 12,
    slot: 8,
    branch: "pricing-copy",
    waiting_for: [{ id: "draft", text: "Approve the pull request description", kind: "draft" }],
    in_flight: [],
  },
  {
    id: "c40d19e2",
    title: "Blog index loads slowly",
    manifest_id: "website",
    turn: "working",
    phase: "read",
    last_turn_at: 0,
    slot: 9,
    branch: "blog-index-speed",
    waiting_for: [],
    in_flight: [],
    forked_from: "Pricing page copy",
  },
  {
    id: "e72a5b10",
    title: "Rotate the staging certificates",
    manifest_id: "infra",
    turn: "idle",
    last_turn_at: 3,
    slot: 11,
    branch: "staging-certs",
    waiting_for: [{ id: "run", text: "Run the certificate push on staging", kind: "run" }],
    in_flight: [],
  },
  {
    id: "f1a2c3d4",
    title: "Backups land in the new bucket",
    manifest_id: "infra",
    turn: "working",
    phase: "run",
    last_turn_at: 0,
    slot: 12,
    branch: "backups-bucket",
    waiting_for: [],
    in_flight: [{ lifecycle: "ticket", key: "INF-41", label: "Backups bucket", state: "in_progress" }],
  },
];

export const ENDED: Job[] = [
  { id: "1a0e9d4f", title: "Should Bridge leave Electron", manifest_id: "armada", turn: "idle", last_turn_at: 1500, branch: "armada/session-1a0e9d4f", waiting_for: [], in_flight: [], ended: true },
  { id: "4c11b0aa", title: "Merge line view in Fleet", manifest_id: "armada", turn: "idle", last_turn_at: 180, branch: "fleet/merge-line", waiting_for: [], in_flight: [], ended: true },
  { id: "77e0c2d1", title: "Mods page walk", manifest_id: "armada", turn: "idle", last_turn_at: 7000, branch: "mods-page", waiting_for: [], in_flight: [], ended: true },
  { id: "a90b1c2e", title: "Pricing table on mobile", manifest_id: "website", turn: "idle", last_turn_at: 2900, branch: "pricing-mobile", waiting_for: [], in_flight: [], ended: true },
];

// A thread, as `JobThread.items` carries it: oldest first.
export type Row =
  | { kind: "message"; id: string; at: string; from: "person" | "agent"; text: string; files?: string[]; tags?: string[] }
  | { kind: "lease"; id: string; at: string; slot: number; branch: string }
  | { kind: "plan"; id: string; at: string; title: string; groups: { title: string; gives: string }[]; asked: boolean }
  | { kind: "draft"; id: string; at: string; title: string; text: string; asked: boolean };

export type Item =
  | { kind: "row"; row: Row }
  | { kind: "calls"; id: string; names: string[]; rows: { id: string; text: string }[] }
  | { kind: "subagent"; id: string; at: string; subagent: { description: string; agent: string; model: string; state: "running" | "finished" | "cut_off" } }
  | { kind: "checks"; id: string; name: string; rows: { at: string; state: "passed" | "failed" | "running" }[] }
  | { kind: "fleet_turn"; id: string; notice: string; rows: { id: string; text: string }[]; handled: boolean }
  | { kind: "sketch"; id: string; at: string; title: string }
  | { kind: "document"; id: string; at: string; title: string; revision: number }
  | { kind: "change"; id: string; at: string; what: "model" | "effort" | "workflow"; from: string; to: string }
  | { kind: "handoff"; id: string; at: string; from: string };

export type Asked = {
  header: string;
  question: string;
  options: { label: string; description: string }[];
};

export type Thread = { items: Item[]; asked?: Asked };

const calls = (id: string, texts: string[]): Item => {
  const names: string[] = [];
  for (const text of texts) {
    const name = text.split(" ")[0];
    if (!names.includes(name)) names.push(name);
  }
  return { kind: "calls", id, names, rows: texts.map((text, n) => ({ id: `${id}-${n}`, text })) };
};

const ORCHESTRATE: Thread = {
  items: [
    { kind: "handoff", id: "h0", at: "08:41", from: "Native Bridge rebuild" },
    { kind: "row", row: { kind: "lease", id: "l0", at: "08:42", slot: 3, branch: "armada/session-20b4e00a" } },
    {
      kind: "row",
      row: {
        kind: "message",
        id: "m0",
        at: "08:43",
        from: "person",
        text: "Start the Jobs screens group once the Fleet views are in. Mocks first.",
      },
    },
    calls("c0", [
      "Read docs/concepts/native-bridge.md",
      "Read crates/ipc/src/job_thread.rs",
      "Grep in_flight crates/ipc/src",
      "Read crates/ipc/src/sessions.rs:1254",
      "Bash git log --oneline origin/main -5",
    ]),
    {
      kind: "row",
      row: {
        kind: "message",
        id: "m1",
        at: "08:47",
        from: "agent",
        text:
          "The Fleet views are on main: the thread rows in #3367 and the in-flight list in #3370. The Foundation Job still owns crates/bridge-native, so the Jobs screens start with mocks and write nothing there until it merges.",
      },
    },
    { kind: "subagent", id: "s0", at: "08:49", subagent: { description: "Survey Electron Jobs screens", agent: "explorer", model: "haiku", state: "finished" } },
    calls("c1", [
      "Bash armada job dispatch title=\"Jobs screens\"",
      "Edit docs/concepts/native-bridge.md",
      "Bash git diff --stat",
    ]),
    { kind: "checks", id: "k0", name: "docs", rows: [{ at: "08:52", state: "failed" }, { at: "08:55", state: "passed" }] },
    {
      kind: "fleet_turn",
      id: "f0",
      notice: "#3370 is in the merge queue.",
      rows: [{ id: "f0-0", text: "Bash gh pr view 3370 --json state" }],
      handled: true,
    },
    {
      kind: "row",
      row: {
        kind: "message",
        id: "m2",
        at: "09:02",
        from: "agent",
        text: "#3370 is queued. Each Job's record now lists what it holds open: tickets, pull requests and linked Jobs, each with its state.",
      },
    },
    { kind: "change", id: "ch0", at: "09:04", what: "model", from: "opus", to: "sonnet" },
    { kind: "sketch", id: "sk0", at: "09:06", title: "Jobs screens build order" },
    calls("c2", [
      "Read docs/practices/native-bridge.md",
      "Bash armada job record_plan",
    ]),
    {
      kind: "row",
      row: {
        kind: "plan",
        id: "p0",
        at: "09:10",
        title: "Jobs screens",
        asked: true,
        groups: [
          { title: "Mocks", gives: "HTML mocks of the list, thread, composer and Needs you, for your walk" },
          { title: "List and switching", gives: "The list grouped by project, and a switch in under 50 ms" },
          { title: "Thread and composer", gives: "The thread with folded tool runs, and the full composer" },
          { title: "Needs you", gives: "Needs you in the list, a banner and sound, and the dock count" },
        ],
      },
    },
  ],
  asked: {
    header: "Delivery",
    question: "How is the work delivered?",
    options: [
      { label: "Auto land", description: "Fleet merges each pull request once ci is green" },
      { label: "Pull request", description: "Each pull request waits for you to merge it" },
      { label: "Branch only", description: "Pushed, with no pull request" },
    ],
  },
};

const FOUNDATION: Thread = {
  items: [
    { kind: "handoff", id: "h0", at: "07:10", from: "Plan and orchestrate the native rebuild" },
    { kind: "row", row: { kind: "lease", id: "l0", at: "07:12", slot: 7, branch: "armada/session-34aa1317" } },
    calls("c0", [
      "Read crates/bridge-native/src/main.rs",
      "Edit crates/bridge-native/src/window.rs",
      "Edit crates/bridge-native/src/theme.rs",
      "Bash cargo check -p bridge-native",
    ]),
    {
      kind: "row",
      row: {
        kind: "message",
        id: "m0",
        at: "07:40",
        from: "agent",
        text: "The window draws the Jobs from Fleet in the theme you picked. A cold build took 4 min 10 s.",
      },
    },
    calls("c1", ["Bash git merge origin/main", "Read crates/fleet/tests/mod.rs", "Edit crates/fleet/tests/mod.rs"]),
    {
      kind: "row",
      row: {
        kind: "message",
        id: "m1",
        at: "13:58",
        from: "agent",
        text: "Merged origin/main. The one conflict was the Fleet test module list, and both sides are kept.",
      },
    },
    calls("c2", ["Bash armada check test native_walk thread_keeping"]),
  ],
};

const PRICING: Thread = {
  items: [
    { kind: "row", row: { kind: "message", id: "m0", at: "10:02", from: "person", text: "Tighten the pricing page copy. Keep the plan names.", tags: ["#212"] } },
    { kind: "row", row: { kind: "lease", id: "l0", at: "10:03", slot: 8, branch: "pricing-copy" } },
    calls("c0", ["Read src/pages/pricing.astro", "Edit src/pages/pricing.astro", "Bash pnpm build"]),
    {
      kind: "row",
      row: {
        kind: "draft",
        id: "d0",
        at: "10:21",
        title: "Pull request description",
        asked: true,
        text: "Shorten the pricing page copy.\n\nEach plan keeps its name. The feature lists lose the repeated lines, and the page is one screen shorter on a laptop.",
      },
    },
  ],
};

const EMPTY: Thread = {
  items: [
    calls("c0", ["Read README.md", "Grep TODO src", "Read src/index.ts"]),
    { kind: "row", row: { kind: "message", id: "m0", at: "11:00", from: "agent", text: "Working through the first file." } },
  ],
};

export const threadOf = (id: string): Thread =>
  id === "20b4e00a" ? ORCHESTRATE : id === "34aa1317" ? FOUNDATION : id === "b31c0a77" ? PRICING : EMPTY;

export const SLASH = [
  { name: "/compact", says: "Compact the conversation" },
  { name: "/clear", says: "Start the conversation again" },
  { name: "/model", says: "Change the model" },
  { name: "/review", says: "Review the branch" },
  { name: "/land", says: "Merge the pull request when ci is green" },
];

export const TAGS = [
  { name: "#3370", says: "In-flight list on a Job's record", kind: "pull_request" },
  { name: "#3368", says: "Ship the workflows as gated plan groups", kind: "pull_request" },
  { name: "armada-a3f2", says: "Native thread folds tool runs", kind: "issue" },
  { name: "armada-mobm", says: "A frame keeps its old theme", kind: "issue" },
];
