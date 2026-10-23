// The Artifacts layer's graph, as data: how a Job grew, each thing hanging off what produced it. Nodes are the person's messages that something
// followed, the plan and its groups, and one node to each artifact (its versions are marks inside it). It is read from the same rows the list
// draws (`artifacts.ts`) and from `Session.versions`, whose `from` Fleet fills: the message a version followed and the plan task in progress.
// The list's search dims what it does not match. Commands, subagents and background tasks are not artifacts and are not here.
//
// Where it is laid out: a column is the order things first appeared, never left of what produced them; a row is packed by walking the tree of
// first sources, so a parent sits among its children and no two nodes overlap.

import { Check, Eye, FilePenLine, GitMerge, GitPullRequest, Layers, ListChecks, MessageSquare, PencilRuler, RefreshCw, ShieldCheck, ShieldEllipsis, ShieldX, Ticket, type LucideIcon } from "lucide-react";
import type { ArtifactGraphEdge, ArtifactGraphMark, ArtifactGraphNode } from "@armada/components";
import { issueLabel, type Session, type SessionAttachment, type SessionRow } from "@armada/screens/src/draft/sessions";

import { shownOnly } from "../session-cards";
import type { ArtifactGroup, ArtifactRow } from "./artifacts";

/** What opening a mark may need that a row does not hold: the sketch as it stood at an instant. */
export type GraphActs = { sketch: (id: string, at?: string) => void; /** A clock time as the person reads it. */ clock: (ms: number) => string };

export type ArtifactGraphModel = { nodes: ArtifactGraphNode[]; edges: ArtifactGraphEdge[] };

export const NODE_WIDTH = 184;
const MAIN = 36;
const MARKS = 32;
const COLUMN_GAP = 36;
const ROW_GAP = 12;

type Version = { at?: string; attachment: SessionAttachment; from?: { message?: string; task?: string } };

const identityOf = (one: SessionAttachment): string | undefined => {
  switch (one.kind) {
    case "pull_request":
      return `pr:${one.number}`;
    case "issue":
      return `issue:${one.number}`;
    case "link":
      return `link:${one.url}`;
    case "sketch":
      return `sketch:${one.id}`;
    case "artifact":
      return `artifact:${one.id}`;
    case "started_to":
      return `session:${one.id}`;
    default:
      return undefined;
  }
};

type What = { said: string; Glyph: LucideIcon; tone?: ArtifactGraphMark["tone"] };

/** What a version did to the one before it, in the words the list's marks use, and the glyph that says it. */
function whatOf(before: SessionAttachment | undefined, now: SessionAttachment, row: ArtifactRow): What {
  switch (now.kind) {
    case "pull_request": {
      if (before?.kind !== "pull_request") return { said: now.state === "draft" ? "Opened as draft" : "Opened", Glyph: GitPullRequest };
      if (now.state !== before.state) return now.state === "merged" ? { said: "Merged", Glyph: GitMerge, tone: "done" } : { said: now.state === "draft" ? "Back to draft" : "Ready for review", Glyph: GitPullRequest };
      if (now.checks.state !== before.checks.state) {
        return now.checks.state === "failed" ? { said: "Checks failed", Glyph: ShieldX, tone: "failed" } : now.checks.state === "pending" ? { said: "Checks running", Glyph: ShieldEllipsis, tone: "pending" } : { said: "Checks passed", Glyph: ShieldCheck, tone: "passed" };
      }
      return { said: "Changed", Glyph: RefreshCw };
    }
    case "issue":
      if (before?.kind !== "issue") return { said: "Added", Glyph: Ticket };
      return now.state === "closed" ? { said: "Closed", Glyph: Check, tone: "done" } : { said: "Reopened", Glyph: Ticket };
    case "sketch":
      return before === undefined ? { said: "Drawn", Glyph: PencilRuler } : { said: "Redrawn", Glyph: RefreshCw };
    case "artifact":
      if (before === undefined) return { said: now.form === "page" || now.form === "doc" ? "Published" : now.form === "window" || shownOnly(now) || now.shown !== undefined ? "Shown" : "Written", Glyph: now.form === "window" || now.form === "page" ? Eye : FilePenLine };
      return { said: now.form === "window" || now.shown !== undefined ? "Shown again" : "Changed", Glyph: RefreshCw };
    case "started_to":
      return { said: row.sub === undefined ? "Started" : `Started from ${row.sub}`, Glyph: row.Glyph };
    default:
      return { said: "Changed", Glyph: RefreshCw };
  }
}

/** A message's first line, cut where it would run on. */
const firstLine = (text: string): string => {
  const line = text.trim().split("\n")[0] ?? "";
  return line.length > 80 ? `${line.slice(0, 79)}…` : line;
};

const sameFrom = (a: Version["from"], b: Version["from"]): boolean => a?.message === b?.message && a?.task === b?.task;

type Draft = {
  node: Omit<ArtifactGraphNode, "x" | "y" | "width" | "height">;
  /** Appearance: milliseconds where known, else the order built. */
  t: number;
  parent?: string;
};

/**
 * The graph of the rows the list shows. `all` is every artifact row, so a search dims a node rather than dropping it, and `shown` is the rows
 * the words matched.
 */
export function graphOf(session: Session, all: readonly ArtifactGroup[], shown: readonly ArtifactGroup[], words: string, acts: GraphActs): ArtifactGraphModel {
  const rows = new Map(all.flatMap((group) => group.rows).map((row) => [row.key, row] as const));
  const lit = new Set(shown.flatMap((group) => group.rows).map((row) => row.key));
  const query = words.trim().toLowerCase();
  const dimmed = (text: string) => query !== "" && !text.toLowerCase().includes(query);
  const history: Version[] = session.versions === undefined ? session.attachments.map((attachment) => ({ attachment })) : session.versions.map((one) => ({ at: one.at, attachment: one.attachment, ...(one.from === undefined ? {} : { from: one.from }) }));

  // Messages of the person's, by row id, with where each stands in the thread.
  const thread = session.rows;
  const person: { row: Extract<SessionRow, { kind: "message" }>; index: number }[] = [];
  thread.forEach((row, index) => {
    if (row.kind === "message" && row.from.kind === "you") person.push({ row, index });
  });
  const messageRow = (id: string | undefined) => (id === undefined ? undefined : person.find((one) => one.row.id === id));
  const firstMessage = person[0];

  const plan = session.work?.plan;
  const groupOfTask = new Map<string, { id: string; title: string; task: string }>();
  for (const group of plan?.groups ?? []) for (const task of group.tasks) groupOfTask.set(task.id, { id: group.id, title: group.title, task: task.text });
  const groupById = new Map((plan?.groups ?? []).map((group) => [group.id, group] as const));

  // The artifacts, each with its versions in order.
  const byKey = new Map<string, Version[]>();
  for (const one of history) {
    const key = identityOf(one.attachment);
    if (key === undefined || !rows.has(key)) continue;
    const held = byKey.get(key);
    if (held === undefined) byKey.set(key, [one]);
    else held.push(one);
  }
  // A row the history does not name appeared with the Job.
  for (const one of session.attachments) {
    const key = identityOf(one);
    if (key !== undefined && rows.has(key) && !byKey.has(key)) byKey.set(key, [{ attachment: one }]);
  }

  const instant = (version: Version | undefined) => (version?.at === undefined ? undefined : Date.parse(version.at));
  let order = 0;
  const drafts = new Map<string, Draft>();
  const edges: ArtifactGraphEdge[] = [];
  const edge = (source: string, target: string, kind: ArtifactGraphEdge["kind"], name: string) => {
    if (source === target || edges.some((one) => one.source === source && one.target === target)) return;
    edges.push({ id: `${source}>${target}`, source, target, kind, name });
  };

  // What a source is called in a tooltip.
  const sourceOf = (from: Version["from"]): { id: string; said: string } | undefined => {
    const task = from?.task === undefined ? undefined : groupOfTask.get(from.task);
    if (task !== undefined) return { id: `group:${task.id}`, said: `From the task ${task.task}` };
    const message = messageRow(from?.message);
    return message === undefined ? undefined : { id: `message:${message.row.id}`, said: `From the message ${firstLine(message.row.text)}` };
  };

  const used = new Set<string>();
  const need = (id: string) => void used.add(id);

  // The plan hangs off the message it followed: the latest of his before the row that recorded it.
  let planParent: string | undefined;
  if (plan !== undefined) {
    const after = plan.after === undefined ? -1 : thread.findIndex((row) => row.id === plan.after);
    const before = after < 0 ? undefined : [...person].reverse().find((one) => one.index < after);
    const message = before ?? firstMessage;
    if (message !== undefined) planParent = `message:${message.row.id}`;
  }

  // Artifacts.
  const artifacts = [...byKey].map(([key, versions]) => {
    const row = rows.get(key)!;
    const first = versions[0]!;
    const started = first.attachment.kind === "started_to" && first.attachment.group !== undefined && groupById.has(first.attachment.group) ? first.attachment.group : undefined;
    const sourced = sourceOf(first.from);
    const parent = started !== undefined ? `group:${started}` : sourced?.id ?? (firstMessage === undefined ? undefined : `message:${firstMessage.row.id}`);
    return { key, row, versions, first, parent, started, sourced };
  });

  for (const one of artifacts) {
    const { key, row, versions, first, parent, started, sourced } = one;
    const marks = versions.length < 2 ? [] : versions.map((version, index): ArtifactGraphMark => {
      const what = whatOf(versions[index - 1]?.attachment, version.attachment, row);
      const at = instant(version);
      const source = sourceOf(version.from);
      const sketch = version.attachment.kind === "sketch" ? version.attachment : undefined;
      const open = sketch === undefined ? row.open : () => acts.sketch(sketch.id, version.at);
      return {
        key: `${key}@${index}`,
        Glyph: what.Glyph,
        ...(what.tone === undefined ? {} : { tone: what.tone }),
        name: `${what.said}, ${row.title}`,
        tip: [what.said, row.title, ...(at === undefined || Number.isNaN(at) ? [] : [acts.clock(at)]), ...(source === undefined ? [] : [source.said])],
        ...(open === undefined ? {} : { open }),
      };
    });
    const startedGroup = started === undefined ? undefined : groupById.get(started);
    const t = instant(first) ?? order;
    drafts.set(key, {
      t: Number.isNaN(t) ? order : t,
      ...(parent === undefined ? {} : { parent }),
      node: {
        id: key,
        kind: "artifact",
        Glyph: row.Glyph,
        name: `${row.said}, ${row.title}`,
        title: row.title,
        tip: [row.title, row.said, ...(startedGroup !== undefined ? [`Started from the group ${startedGroup.title}`] : sourced === undefined ? [] : [sourced.said])],
        dim: !lit.has(key),
        marks,
        ...(row.open === undefined ? {} : { open: row.open }),
      },
    });
    order += 1;
    if (parent !== undefined) {
      need(parent);
      edge(parent, key, "from", startedGroup !== undefined ? `Started from the group ${startedGroup.title}` : sourced !== undefined ? sourced.said : `From the message ${firstLine(firstMessage?.row.text ?? "")}`);
    }
    // A later version that followed something else draws a second, thin edge from it.
    for (const [index, version] of versions.entries()) {
      if (index === 0 || sameFrom(version.from, first.from)) continue;
      const source = sourceOf(version.from);
      if (source === undefined) continue;
      need(source.id);
      edge(source.id, key, "again", `${whatOf(versions[index - 1]?.attachment, version.attachment, row).said}, ${source.said.charAt(0).toLowerCase()}${source.said.slice(1)}`);
    }
  }

  // The joins already known: an issue to the pull request that closes it.
  for (const one of artifacts) {
    for (const version of one.versions) {
      const now = version.attachment;
      if (now.kind === "pull_request") {
        for (const number of now.closes ?? []) {
          if (drafts.has(`issue:${number}`)) edge(`issue:${number}`, one.key, "join", `Closes ${issueLabel(String(number))}`);
        }
      }
      if (now.kind === "issue") {
        for (const number of now.pull_requests ?? []) {
          if (drafts.has(`pr:${number}`)) edge(`issue:${now.number}`, `pr:${number}`, "join", `Closes ${issueLabel(now.number)}`);
        }
      }
    }
  }

  // The plan and its groups, where something hangs off them or the plan stands.
  if (plan !== undefined) {
    for (const group of plan.groups) need(`group:${group.id}`);
    need("plan");
    if (planParent !== undefined) need(planParent);
  }

  for (const id of used) {
    if (id === "plan" || drafts.has(id)) continue;
    if (id.startsWith("message:")) {
      const found = person.find((one) => `message:${one.row.id}` === id);
      if (found === undefined) continue;
      const title = firstLine(found.row.text);
      drafts.set(id, { t: Date.parse(found.row.at), node: { id, kind: "message", Glyph: MessageSquare, name: `Message, ${title}`, title, tip: [title, "Message"], dim: dimmed(title), marks: [] } });
    }
  }
  if (plan !== undefined) {
    const planAt = plan.after === undefined ? undefined : Date.parse(thread.find((row) => row.id === plan.after)?.at ?? "");
    const title = "Plan";
    const version = plan.version === undefined ? undefined : `v${plan.version}`;
    drafts.set("plan", {
      t: planAt !== undefined && !Number.isNaN(planAt) ? planAt : -Infinity,
      ...(planParent === undefined ? {} : { parent: planParent }),
      node: { id: "plan", kind: "plan", Glyph: ListChecks, name: version === undefined ? "Plan" : `Plan, version ${plan.version}`, title, ...(version === undefined ? {} : { sub: version }), tip: [title, version === undefined ? "The plan" : `The plan, version ${plan.version}`], dim: dimmed(title), marks: [] },
    });
    if (planParent !== undefined) edge(planParent, "plan", "from", "The plan followed this message");
    for (const group of plan.groups) {
      drafts.set(`group:${group.id}`, {
        t: -Infinity,
        parent: "plan",
        node: { id: `group:${group.id}`, kind: "group", Glyph: Layers, name: `Group, ${group.title}`, title: group.title, tip: [group.title, "Group of the plan"], dim: dimmed(`${group.title} ${group.agent}`), marks: [] },
      });
      edge("plan", `group:${group.id}`, "from", `Plan to group ${group.title}`);
    }
  }

  // A node with no time of its own appears when its parent does; a message with none comes just before what first followed it. Never before what produced it.
  const ids = [...drafts.keys()];
  const resolved = new Map<string, number>();
  const timeOf = (id: string, seen: ReadonlySet<string>): number => {
    const held = resolved.get(id);
    if (held !== undefined) return held;
    const draft = drafts.get(id)!;
    const down = new Set(seen).add(id);
    const parent = draft.parent !== undefined && drafts.has(draft.parent) && !seen.has(draft.parent) ? timeOf(draft.parent, down) : undefined;
    let t = draft.t;
    if (!Number.isFinite(t)) {
      const kids = ids.flatMap((it) => (drafts.get(it)!.parent === id && Number.isFinite(drafts.get(it)!.t) ? [drafts.get(it)!.t] : []));
      t = parent ?? (kids.length === 0 ? 0 : Math.min(...kids) - 1);
    }
    if (parent !== undefined) t = Math.max(t, parent);
    resolved.set(id, t);
    return t;
  };
  for (const id of ids) timeOf(id, new Set());

  return layOut(drafts, edges, resolved);
}

const heightOf = (node: { marks: readonly unknown[] }) => (node.marks.length === 0 ? MAIN : MAIN + MARKS);

/** Columns by first appearance, never left of the parent; rows by walking the tree of first sources. */
function layOut(drafts: ReadonlyMap<string, Draft>, edges: readonly ArtifactGraphEdge[], times: ReadonlyMap<string, number>): ArtifactGraphModel {
  const ids = [...drafts.keys()];
  const index = new Map(ids.map((id, at) => [id, at] as const));
  const parentOf = (id: string) => {
    const parent = drafts.get(id)!.parent;
    return parent !== undefined && drafts.has(parent) ? parent : undefined;
  };
  const depth = new Map<string, number>();
  const depthOf = (id: string): number => {
    const held = depth.get(id);
    if (held !== undefined) return held;
    depth.set(id, 0);
    const parent = parentOf(id);
    const found = parent === undefined ? 0 : depthOf(parent) + 1;
    depth.set(id, found);
    return found;
  };
  ids.forEach(depthOf);
  const byTime = [...ids].sort((a, b) => times.get(a)! - times.get(b)! || depth.get(a)! - depth.get(b)! || index.get(a)! - index.get(b)!);

  const column = new Map<string, number>();
  let last = 0;
  for (const id of byTime) {
    const parent = parentOf(id);
    const at = Math.max(depth.get(id)!, last, parent === undefined ? 0 : (column.get(parent) ?? 0) + 1);
    column.set(id, at);
    last = at;
  }

  const children = new Map<string, string[]>();
  const roots: string[] = [];
  for (const id of byTime) {
    const parent = parentOf(id);
    if (parent === undefined) roots.push(id);
    else children.set(parent, [...(children.get(parent) ?? []), id]);
  }
  const top = new Map<string, number>();
  let cursor = 0;
  /** Places a subtree in the rows from the cursor down and returns what it placed. A parent taller than the band its children make pushes the whole of it down. */
  const place = (id: string): string[] => {
    const own = heightOf(drafts.get(id)!.node);
    const kids = children.get(id) ?? [];
    const start = cursor;
    if (kids.length === 0) {
      top.set(id, cursor);
      cursor += own + ROW_GAP;
      return [id];
    }
    const placed = kids.flatMap(place);
    const first = kids[0]!;
    const final = kids[kids.length - 1]!;
    const middle = (top.get(first)! + heightOf(drafts.get(first)!.node) / 2 + top.get(final)! + heightOf(drafts.get(final)!.node) / 2) / 2;
    const at = middle - own / 2;
    const shift = Math.max(0, start - at);
    for (const it of placed) top.set(it, top.get(it)! + shift);
    top.set(id, at + shift);
    cursor = Math.max(cursor + shift, at + shift + own + ROW_GAP);
    return [id, ...placed];
  };
  roots.forEach(place);

  const nodes = ids.map((id): ArtifactGraphNode => {
    const draft = drafts.get(id)!;
    return { ...draft.node, x: column.get(id)! * (NODE_WIDTH + COLUMN_GAP), y: top.get(id)!, width: NODE_WIDTH, height: heightOf(draft.node) };
  });
  return { nodes, edges: [...edges] };
}

