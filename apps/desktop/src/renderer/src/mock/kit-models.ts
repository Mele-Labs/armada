// Mock-only: what Fleet's `list_models` answers, held in memory. The four models Claude Code offers, the
// ones Jobs may use, and the model each kind of run starts on.

import type { KitModel, KitModelChoices, KitModelsRead } from "@armada/components";

import type { KitModelsSource } from "../kit-models";

const LEVELS = ["low", "medium", "high", "xhigh", "max"];

const MODELS: KitModel[] = [
  { name: "opus", display: "Opus", resolved: "claude-opus-5-5", says: "For complex work and everyday tasks", effort_levels: LEVELS, on: true },
  { name: "fable", display: "Fable", resolved: "claude-fable-5-1", says: "For your toughest challenges", effort_levels: LEVELS, on: true },
  { name: "sonnet", display: "Sonnet", resolved: "claude-sonnet-5-5", says: "Most efficient for simpler tasks", effort_levels: LEVELS.slice(0, 4), on: true },
  { name: "haiku", display: "Haiku", resolved: "claude-haiku-4-5", says: "Fastest for quick answers", effort_levels: LEVELS.slice(0, 3), on: false },
];

const CHOICES: KitModelChoices = { jobs: "opus", drones: "sonnet", reviews: "opus", effort: "high" };

export function createMockKitModels(): KitModelsSource {
  let held: KitModelsRead = { read: "read", models: MODELS, choices: CHOICES };
  const listeners = new Set<() => void>();
  const set = (next: KitModelsRead) => {
    held = next;
    listeners.forEach((on) => on());
  };
  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setOn: (name, on) => {
      if (held.read !== "read") return;
      const models = held.models.map((one) => (one.name === name ? { ...one, on } : one));
      const offered = models.filter((one) => one.on);
      const fallback = offered[0]?.name;
      // A run that started on a model now off starts on the first still on.
      const keep = (current: string) => (fallback === undefined || offered.some((one) => one.name === current) ? current : fallback);
      const { jobs, drones, reviews, effort } = held.choices;
      const levels = new Set(offered.flatMap((one) => one.effort_levels));
      set({ read: "read", models, choices: { jobs: keep(jobs), drones: keep(drones), reviews: keep(reviews), effort: levels.has(effort) ? effort : (offered[0]?.effort_levels[0] ?? effort) } });
    },
    choose: (slot, value) => {
      if (held.read !== "read") return;
      set({ ...held, choices: { ...held.choices, [slot]: value } });
    },
  };
}
