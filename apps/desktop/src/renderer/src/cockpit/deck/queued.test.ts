// Where a queued idea sits in the deck's list: in its repository's group ahead of that group's rows, in the flat list's head
// where there are no groups, and in the order the keys walk.

import { expect, it } from "vitest";
import type { RepositorySummary } from "@armada/protocol";
import type { QueuedIdea } from "@armada/screens/src/draft/sessions";

import { bandsOf, OTHER_BAND, type Banding } from "./bands";
import { queuedIn, queuedKey, walkOrder } from "./queued";
import type { Subject } from "./ship";

const banding: readonly Banding[] = [
  { manifest: "armada", root: "/code/armada", name: "armada" },
  { manifest: "billing", root: "/code/billing", name: "billing" },
  { manifest: "docs", root: "/code/docs", name: "docs" },
];

const session = (id: string, manifest: string | undefined): Subject => ({ key: `session:${id}`, kind: "session", id, ...(manifest === undefined ? {} : { manifest }) }) as unknown as Subject;
const idea = (id: string, root: string): QueuedIdea => ({ id, text: `Idea ${id}`, root, repository: root.split("/").pop()! });
const repository = (root: string) => ({ root }) as unknown as RepositorySummary;

it("seats an idea in its repository's group, ahead of that group's rows", () => {
  const bands = bandsOf([session("a", "armada"), session("b", "billing")], banding, [idea("q1", "/code/billing")]);
  expect(bands.map((one) => one.label)).toEqual(["armada", "billing"]);
  expect(bands[0]!.ideas).toEqual([]);
  expect(bands[1]!.ideas.map((one) => one.id)).toEqual(["q1"]);
  expect(bands[1]!.subjects.map((one) => one.key)).toEqual(["session:b"]);
});

it("gives a repository that holds only ideas its own group, and leaves one with nothing out", () => {
  const bands = bandsOf([session("a", "armada")], banding, [idea("q1", "/code/docs")]);
  expect(bands.map((one) => one.label)).toEqual(["armada", "docs"]);
  expect(bands[1]!.subjects).toEqual([]);
  expect(bands.some((one) => one.label === "billing")).toBe(false);
});

it("puts an idea whose repository is not listed in Other, with the rows that have none", () => {
  const bands = bandsOf([session("x", undefined)], banding, [idea("q1", "/code/elsewhere")]);
  expect(bands.map((one) => one.label)).toEqual([OTHER_BAND]);
  expect(bands[0]!.ideas.map((one) => one.id)).toEqual(["q1"]);
});

it("keeps the order ideas were queued in within a group", () => {
  const bands = bandsOf([], banding, [idea("q1", "/code/armada"), idea("q2", "/code/billing"), idea("q3", "/code/armada")]);
  expect(bands[0]!.ideas.map((one) => one.id)).toEqual(["q1", "q3"]);
});

it("draws the groups it always did where nothing is queued", () => {
  expect(bandsOf([session("a", "armada")], banding).map((one) => [one.label, one.ideas.length])).toEqual([["armada", 0]]);
});

it("narrows ideas to the picked repository, and shows all where none is picked", () => {
  const ideas = [idea("q1", "/code/armada"), idea("q2", "/code/billing")];
  expect(queuedIn(ideas, null)).toEqual(ideas);
  expect(queuedIn(ideas, repository("/code/billing")).map((one) => one.id)).toEqual(["q2"]);
});

it("walks each group's ideas before its rows, group by group", () => {
  const listed = [session("a", "armada"), session("b", "billing")];
  const ideas = [idea("q1", "/code/billing"), idea("q2", "/code/armada")];
  expect(walkOrder(listed, banding, ideas, null)).toEqual([queuedKey("q2"), "session:a", queuedKey("q1"), "session:b"]);
});

it("walks the ideas, then the rows, where there are no groups", () => {
  const listed = [session("a", "armada")];
  const ideas = [idea("q1", "/code/armada"), idea("q2", "/code/billing")];
  expect(walkOrder(listed, undefined, ideas, null)).toEqual([queuedKey("q1"), queuedKey("q2"), "session:a"]);
  expect(walkOrder(listed, undefined, ideas, repository("/code/billing"))).toEqual([queuedKey("q2"), "session:a"]);
});
