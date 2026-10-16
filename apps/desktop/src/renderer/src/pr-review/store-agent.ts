// **The mock-backed half of the review store: everything an agent would do.** A quick action, Ask, a review tool and
// Natural Search each "run" for a moment and come to what a fixture says (`AgentFixtures`). Fleet serves none of it, and
// wave 2 replaces this file with runs; the store (`store.tsx`) reaches it only through `AgentActs`, so nothing else
// changes. An agent runs only when he presses, and asks first: `spend` is the one gate.

import type { AgentFixtures, Asked, Find, Finding, PrWork, ReviewPr, ReviewSnapshot, ToolDef } from "./model";
import type { ToolIn } from "./store";
import { bySession, idOfName } from "./tools";

/** How long an agent runs in the mock before it has read the pull request. */
export const AGENT_MS = 2200;

/** What the agent half reads and moves of the store. */
export type StoreHold = {
  get: () => ReviewSnapshot;
  set: (next: Partial<ReviewSnapshot>) => void;
  work: (key: string) => PrWork;
  change: (key: string, by: Partial<PrWork>) => void;
  prOf: (key: string) => ReviewPr | undefined;
  timers: Set<number>;
  nextId: (prefix: string) => string;
};

export type AgentActs = {
  find: (words: string) => void;
  rerunFind: (id: string) => void;
  removeFind: (id: string) => void;
  spend: (what: string, run: () => void, cancelled?: () => void) => void;
  confirmSpend: (again: boolean) => void;
  cancelSpend: () => void;
  explain: (key: string, def: ToolDef) => void;
  explainAgain: (key: string, def: ToolDef) => void;
  show: (key: string, kind: string) => void;
  ask: (key: string, words: string) => void;
  askAgain: (key: string, id: string) => void;
  removeAsk: (key: string, id: string) => void;
  runTool: (key: string, tool: ToolDef) => void;
  rerun: (key: string, tool: ToolDef) => void;
  dismiss: (key: string, finding: string) => void;
  sent: (key: string, findings: readonly string[]) => void;
  addTool: (one: ToolIn) => void;
  /** What a pull request starts with: the explanations a Session of his already knows and the questions already asked. */
  initial: (pr: ReviewPr) => Pick<PrWork, "explains" | "kind" | "asks">;
};

export function createAgentActs(agent: AgentFixtures, hold: StoreHold): AgentActs {
  const { get, set, work, change, prOf, timers, nextId } = hold;
  let waiting: (() => void) | undefined;
  let declined: (() => void) | undefined;

  const setFind = (id: string, by: Partial<Find>) => set({ finds: get().finds.map((one) => (one.id === id ? { ...one, ...by } : one)) });
  const look = (id: string, words: string) => {
    setFind(id, { run: { state: "running", since: Date.now() }, hits: [] });
    timers.add(
      window.setTimeout(() => {
        const hits = agent.finds.find((one) => new RegExp(one.when, "i").test(words))?.hits ?? [];
        setFind(id, { run: { state: "done", at: Date.now(), head: "" }, hits });
      }, AGENT_MS + 600),
    );
  };

  const start = (key: string, tool: ToolDef) => {
    const head = prOf(key)?.head ?? "";
    change(key, {
      view: tool.id,
      runs: { ...work(key).runs, [tool.id]: { state: "running", since: Date.now() } },
      findings: work(key).findings.filter((one) => one.tool !== tool.id),
    });
    timers.add(
      window.setTimeout(() => {
        const found: Finding[] = (agent.results[key]?.[tool.id] ?? []).map((one) => ({ id: nextId("f"), tool: tool.id, ...one, dismissed: false, sent: false }));
        change(key, { runs: { ...work(key).runs, [tool.id]: { state: "done", at: Date.now(), head } }, findings: [...work(key).findings, ...found] });
      }, AGENT_MS),
    );
  };

  const read = (key: string, def: ToolDef, head: string) => {
    change(key, { kind: def.id, explains: { ...work(key).explains, [def.id]: { state: "running", since: Date.now() } } });
    timers.add(window.setTimeout(() => change(key, { explains: { ...work(key).explains, [def.id]: { state: "done", at: Date.now(), head } } }), AGENT_MS));
  };

  const setAsk = (key: string, id: string, run: Asked["run"]) => change(key, { asks: work(key).asks.map((one) => (one.id === id ? { ...one, run } : one)) });
  const answer = (key: string, id: string, head: string) => {
    setAsk(key, id, { state: "running", since: Date.now() });
    timers.add(window.setTimeout(() => setAsk(key, id, { state: "done", at: Date.now(), head }), AGENT_MS));
  };

  return {
    initial: (pr) => ({
      explains: pr.explainedBy === "session" ? ({ "how-it-works": { state: "done", at: Date.now(), head: pr.head }, why: { state: "done", at: Date.now(), head: pr.head } } as PrWork["explains"]) : {},
      kind: pr.explainedBy === "session" ? "how-it-works" : undefined,
      asks: pr.asked.map((words) => ({ id: nextId("ask"), words, run: { state: "done", at: Date.now(), head: pr.head } })),
    }),
    find: (words) => {
      const said = words.trim().replace(/\s+/g, " ");
      if (said === "") return;
      const snapshot = get();
      const again = snapshot.finds.find((one) => one.words.toLowerCase() === said.toLowerCase());
      const id = again?.id ?? nextId("find");
      if (again === undefined) set({ finds: [...snapshot.finds, { id, words: said, run: { state: "idle" }, hits: [] }] });
      set({ tab: id });
      if (again?.run.state !== "running") look(id, said);
    },
    rerunFind: (id) => {
      const one = get().finds.find((one) => one.id === id);
      if (one !== undefined && one.run.state !== "running") look(id, one.words);
    },
    removeFind: (id) => set({ finds: get().finds.filter((one) => one.id !== id), tab: get().tab === id ? "requested" : get().tab }),
    spend: (what, run, cancelled) => {
      if (!get().askFirst) return run();
      waiting = run;
      declined = cancelled;
      set({ spending: what });
    },
    confirmSpend: (again) => {
      const run = waiting;
      waiting = undefined;
      declined = undefined;
      set({ spending: undefined, ...(again ? { askFirst: false } : {}) });
      run?.();
    },
    cancelSpend: () => {
      const no = declined;
      waiting = undefined;
      declined = undefined;
      set({ spending: undefined });
      no?.();
    },
    explain: (key, def) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      const kept = work(key).explains[def.id];
      change(key, { kind: def.id });
      if (bySession(pr, def) || (kept !== undefined && kept.state !== "idle")) return;
      read(key, def, pr.head);
    },
    explainAgain: (key, def) => {
      const pr = prOf(key);
      if (pr === undefined || bySession(pr, def) || work(key).explains[def.id]?.state === "running") return;
      read(key, def, pr.head);
    },
    show: (key, kind) => change(key, { kind }),
    ask: (key, words) => {
      const pr = prOf(key);
      const said = words.trim().replace(/\s+/g, " ");
      if (pr === undefined || said === "") return;
      const again = work(key).asks.find((one) => one.words.toLowerCase() === said.toLowerCase());
      const id = again?.id ?? nextId("ask");
      if (again === undefined) change(key, { asks: [...work(key).asks, { id, words: said, run: { state: "idle" } }] });
      change(key, { kind: id });
      if (again?.run.state !== "running") answer(key, id, pr.head);
    },
    askAgain: (key, id) => {
      const pr = prOf(key);
      if (pr !== undefined && work(key).asks.find((one) => one.id === id)?.run.state !== "running") answer(key, id, pr.head);
    },
    removeAsk: (key, id) => change(key, { asks: work(key).asks.filter((one) => one.id !== id), kind: work(key).kind === id ? undefined : work(key).kind }),
    runTool: (key, tool) => {
      if (!tool.agent) return change(key, { view: tool.id });
      const run = work(key).runs[tool.id];
      if (run === undefined || run.state === "idle") return start(key, tool);
      change(key, { view: tool.id });
    },
    rerun: (key, tool) => {
      if (work(key).runs[tool.id]?.state === "running") return;
      start(key, tool);
    },
    dismiss: (key, finding) => change(key, { findings: work(key).findings.map((one) => (one.id === finding ? { ...one, dismissed: true } : one)) }),
    sent: (key, findings) => change(key, { findings: work(key).findings.map((one) => (findings.includes(one.id) ? { ...one, sent: true } : one)) }),
    addTool: ({ kind, name, checks, scope, manifestId }) => {
      const def: ToolDef = { id: idOfName(name), kind, name: name.trim(), source: scope, mark: "custom", checks: checks.trim(), agent: true };
      const snapshot = get();
      if (scope === "kit") set({ kit: [...snapshot.kit.filter((one) => one.id !== def.id), def] });
      else set({ repository: { ...snapshot.repository, [manifestId]: [...(snapshot.repository[manifestId] ?? []).filter((one) => one.id !== def.id), def] } });
    },
  };
}
