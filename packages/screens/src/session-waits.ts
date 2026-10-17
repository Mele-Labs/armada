// What a Session that waits on another says of itself: when it starts, and from where. One reading for the Work list's
// tooltip, the waiting Session's page, the dispatch panel and the rows in the other Session's thread. Draft, mock-only
// (`draft/sessions.ts`).

import type { Session, SessionWait, StartWhen } from "./draft/sessions";

/** The four conditions, with the option each is picked by and where the Session starts from. **The other Session ending is the default.** */
export const START_WHEN: readonly { id: StartWhen; label: string; from: string }[] = [
  { id: "ends", label: "It ends", from: "main, with its handoff" },
  { id: "plan", label: "A plan part is done", from: "main" },
  { id: "pushed", label: "Its pull request is pushed", from: "that pull request's branch" },
  { id: "lands", label: "Its pull request lands", from: "main, after the merge" },
];

export const DEFAULT_START_WHEN: StartWhen = "ends";

/** The condition as the option names it. */
export const whenLabel = (when: StartWhen): string => START_WHEN.find((one) => one.id === when)!.label;

/** Where it starts from: the branch of the pull request once the other Session has named one. */
export function startsFrom(wait: Pick<SessionWait, "when" | "pull">): string {
  if (wait.when === "pushed" && wait.pull !== undefined) return wait.pull.branch;
  return START_WHEN.find((one) => one.id === wait.when)!.from;
}

/** What a waiting Session waits for, as the tooltip on its mark and the line under its title say it. */
export function startsWhenSaid(wait: Pick<SessionWait, "on" | "when" | "part" | "pull">): string {
  switch (wait.when) {
    case "ends":
      return `Starts when ${wait.on.title} ends`;
    case "plan":
      return wait.part === undefined ? `Starts when a plan part of ${wait.on.title} is done` : `Starts when ${wait.part.title} is done in ${wait.on.title}`;
    case "pushed":
      return wait.pull === undefined ? `Starts when ${wait.on.title} pushes a pull request` : `Starts when #${wait.pull.number} is pushed`;
    case "lands":
      return wait.pull === undefined ? `Starts when the pull request of ${wait.on.title} lands` : `Starts when #${wait.pull.number} lands`;
  }
}

/** A group or a task of a Session's plan, for the condition that waits on one. */
export type PlanPart = { id: string; title: string; kind: "group" | "task" };

/** The groups and tasks of a Session's plan, each group followed by its tasks. Empty where the Session has no plan. */
export function planPartsOf(session: Session | undefined): PlanPart[] {
  return (session?.work?.plan?.groups ?? []).flatMap((group) => [
    { id: group.id, title: group.title, kind: "group" as const },
    ...group.tasks.map((task) => ({ id: task.id, title: task.text, kind: "task" as const })),
  ]);
}

/** Whether a part of the plan is done: a task is done, a group when every task in it is. */
export function partDone(session: Session, part: { id: string }): boolean {
  const groups = session.work?.plan?.groups ?? [];
  const group = groups.find((one) => one.id === part.id);
  if (group !== undefined) return group.tasks.length > 0 && group.tasks.every((one) => one.state === "done");
  return groups.some((one) => one.tasks.some((task) => task.id === part.id && task.state === "done"));
}

/** The one line of a brief. */
export const firstLineOf = (words: string): string => words.trim().split("\n")[0]?.trim() ?? "";

/** The condition as a row says it, for what it was and what it is: the option, then the part or the pull request it names. */
export function conditionSaid(wait: Pick<SessionWait, "when" | "part" | "pull">): string {
  const named = wait.when === "plan" ? wait.part?.title : wait.pull === undefined ? undefined : `#${wait.pull.number}`;
  return named === undefined ? whenLabel(wait.when) : `${whenLabel(wait.when)}, ${named}`;
}
