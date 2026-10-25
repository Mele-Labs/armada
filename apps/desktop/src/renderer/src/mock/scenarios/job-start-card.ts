// A Job dispatched from the Dispatch sheet opens held before any agent moves. The proposer reads the request for a moment, then the
// title, Kind and Lands fill in. Changing Kind moves Lands to the usual for that kind, and words that name how it lands win over the usual.
// Start begins the agent. Over `quick-dispatch`, and the walk `job-start-card` plays it.

import type { DeliveryChoice } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionRow, WorkflowChoice } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s210QuickDispatch } from "./quick-dispatch";
import { others, sessionsStore } from "../sessions/script";

/** The first Job dispatched in the mock takes this id. The usual Sessions are left without it, so the new Job is the only one under it. */
const MINE = "s7";

/** How long the proposer reads before the picks fill in. */
const READ_MS = 1200;
/** How long after Start the agent's first rows land. */
const BEGIN_MS = 900;

/** The repository's workflows. Review has no landing step, so a Job of that kind is not asked how it lands. */
const WORKFLOWS: readonly WorkflowChoice[] = [
  { id: "feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] },
  { id: "bug", name: "Bug", steps: ["Reproduce", "Fix", "Verify", "Pull request"] },
  { id: "review", name: "Review", steps: ["Read", "Review", "Report"] },
];

/** What he usually picks for each kind in this repository. */
const USUAL: Readonly<Record<string, DeliveryChoice>> = { feature: "auto_land", bug: "open_pr" };

/** The kind a request reads as. */
const kindOf = (words: string): string => (/\b(bug|broken|crash|fails?|failing|error|regression)\b/i.test(words) ? "bug" : "feature");

/** How the request itself says the work lands, if it does. */
function namedIn(words: string): DeliveryChoice | undefined {
  if (/\bkeep (it )?local\b/i.test(words)) return "keep_local";
  if (/\b(open (a )?pr|pr only)\b/i.test(words)) return "open_pr";
  if (/\b(land it|auto.?land)\b/i.test(words)) return "auto_land";
  return undefined;
}

/** The title the proposer writes: the request's first words. */
const titleOf = (words: string): string => words.trim().split(/\s+/).slice(0, 8).join(" ");

const message = (id: string, text: string): SessionRow => ({ id, at: "14:12:00", kind: "message", from: { kind: "you" }, text });
const tool = (id: string, text: string): SessionRow => ({ id, at: "14:12:09", kind: "tool", text });

export const s323JobStartCard: Scenario = {
  ...s210QuickDispatch,
  name: "job-start-card",
  says: "A Job dispatched from the Dispatch sheet opens held: title, kind, how it lands, model and effort settled before the agent starts",
  draft: {
    ...s210QuickDispatch.draft,
    sessions: () => {
      const store = sessionsStore([], [], new Map([[repository().root, repository().manifest!.id]]), others().filter((one) => one.id !== MINE));
      /** What each held Job's request named, by id. */
      const named = new Map<string, DeliveryChoice>();
      /** The first message each held Job will begin with: his words, then his edit. It is in the thread from Start. */
      const first = new Map<string, string>();
      const startFor = (id: string, kind: string | null): NonNullable<Session["start"]> => {
        const choice = named.get(id);
        const usual = kind === null ? undefined : USUAL[kind];
        return {
          asksDelivery: kind !== "review",
          message: first.get(id) ?? "",
          ...(choice !== undefined ? { picked: { choice, from: "named" as const } } : usual === undefined || kind === "review" ? {} : { picked: { choice: usual, from: "usual" as const } }),
        };
      };
      const timers = (ms: number, run: () => void) => void window.setTimeout(run, ms);
      return {
        ...store,
        start(tag, root, request, tuning, after, held) {
          const id = store.start(tag, root, request, tuning, after);
          if (held !== true || after !== undefined || typeof id !== "string") return id;
          const words = request ?? "";
          const choice = namedIn(words);
          if (choice !== undefined) named.set(id, choice);
          first.set(id, words);
          // The proposer is reading: the Job is up with nothing written.
          store.edit(id, (one) => ({ ...one, start: { asksDelivery: true, message: words }, work: { proposal: { workflow: null, reading: true } } }));
          timers(READ_MS, () => {
            const kind = kindOf(words);
            store.edit(id, (one) => ({
              ...one,
              title: titleOf(words),
              ...(one.model === undefined ? { model: "opus" } : {}),
              ...(one.effort === undefined ? { effort: "high" } : {}),
              start: startFor(id, kind),
              work: {
                proposal: {
                  workflow: kind,
                  picked: ["workflow", ...(one.model === undefined ? ["model" as const] : []), ...(one.effort === undefined ? ["effort" as const] : [])],
                },
              },
            }));
          });
          return id;
        },
        // His words are the first message, held in the composer: the thread has nothing until Start.
        send(id, sent) {
          if (store.get().find((one) => one.id === id)?.start === undefined) return store.send(id, sent);
          first.set(id, sent.text);
          store.edit(id, (one) => ({ ...one, start: { ...one.start!, message: sent.text } }));
        },
        begin(id, delivery, said) {
          const opening = said ?? first.get(id) ?? "";
          store.edit(id, ({ start: _start, ...one }) => {
            const flow = WORKFLOWS.find((it) => it.id === one.work?.proposal?.workflow);
            return {
              ...one,
              rows: opening.trim() === "" ? one.rows : [...one.rows, message(`m${one.rows.length + 1}`, opening)],
              turn: { state: "working" },
              ...(delivery === "auto_land" ? { autoLand: true as const } : {}),
              work: { ...one.work, ...(flow === undefined ? {} : { workflow: { name: flow.name, steps: flow.steps, at: 0 } }) },
            };
          });
          // The agent's first rows follow a moment later. Nothing is asked of him.
          timers(BEGIN_MS, () =>
            store.edit(id, (one) => ({
              ...one,
              rows: [...one.rows, tool(`t${one.rows.length + 1}`, "Read crates/store/src/client.rs"), tool(`t${one.rows.length + 2}`, "Read crates/store/src/retry.rs")],
            })),
          );
        },
        work: {
          workflows: WORKFLOWS,
          // Fleet works the landing choice out again for the new kind.
          choose(id, workflow) {
            store.edit(id, (one) => ({
              ...one,
              ...(one.start === undefined ? {} : { start: startFor(id, workflow) }),
              work: { ...one.work, proposal: { workflow, picked: (one.work?.proposal?.picked ?? []).filter((it) => it !== "workflow") } },
            }));
          },
          unpick(id, value) {
            store.edit(id, (one) =>
              one.work?.proposal === undefined ? one : { ...one, work: { ...one.work, proposal: { ...one.work.proposal, picked: (one.work.proposal.picked ?? []).filter((it) => it !== value) } } },
            );
          },
          approve: () => undefined,
          suggest: () => undefined,
          sendConcern: () => undefined,
          dismissConcern: () => undefined,
        },
      };
    },
  },
};
