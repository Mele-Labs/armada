// The deck's acts are Fleet's or they are not offered: nothing is done locally and said to have happened.

import type { SessionsDraft } from "@armada/screens/src/draft/sessions";
import { expect, test, vi } from "vitest";

import { actsOf } from "./acts";
import type { Pr, Subject } from "./ship";

const opened = vi.hoisted(() => vi.fn(() => true));
vi.mock("../../shared-panel/store", () => ({ openPullRequestInPanel: opened }));

const pr = { key: "pr:/r:7", root: "/r", number: 7, title: "t", branch: "b", url: "https://example.test/7", ci: "passed", stage: "ready" } as Pr;
const subject = (kind: "session" | "pr", extra: Partial<Subject> = {}): Subject => ({ key: `${kind}:x1`, kind, id: "x1", title: "t", root: "/r", repo: "r", pr, ...extra }) as Subject;

const hosts = () => ({ onOpenSession: vi.fn() });
const draft = (extra: Partial<SessionsDraft> = {}) => ({ send: vi.fn(), act: vi.fn(async () => true), stop: vi.fn(), ...extra }) as unknown as SessionsDraft;

test("Reply sends the words to the Session through its own box's route", () => {
  const sessions = draft();
  actsOf(sessions, hosts()).reply(subject("session"), "go on");
  expect(sessions.send).toHaveBeenCalledWith("x1", { text: "go on", files: [], sketches: [], tags: [] });
});

test("Stop is Fleet's for a Session, and not offered, nor faked, where Fleet has none", () => {
  const sessions = draft();
  const acts = actsOf(sessions, hosts());
  expect(acts.can.stop(subject("session"))).toBe(true);
  acts.stop(subject("session"));
  expect(sessions.stop).toHaveBeenCalledWith("x1");

  const bare = actsOf(draft({ stop: undefined }), hosts());
  expect(bare.can.stop(subject("session"))).toBe(false);
  expect(bare.can.stop(subject("pr"))).toBe(false);
  expect(() => bare.stop(subject("session"))).not.toThrow();
});

test("Merge is the by-number act for a Session, and never marks a pull request landed", () => {
  const sessions = draft();
  const acts = actsOf(sessions, hosts());
  acts.merge(subject("session"));
  expect(sessions.act).toHaveBeenCalledWith("x1", 7, "merge");
  expect(pr.stage).toBe("ready");
});

test("Merge is left off where Fleet has none: a pull request nobody owns, a Session without a draft", () => {
  const acts = actsOf(draft(), hosts());
  expect(acts.can.merge(subject("session"))).toBe(true);
  expect(acts.can.merge(subject("pr"))).toBe(false);
  expect(actsOf(undefined, hosts()).can.merge(subject("session"))).toBe(false);
  expect(acts.can.merge(subject("session", { pr: undefined }))).toBe(false);
  expect(() => acts.merge(subject("pr"))).not.toThrow();
});

test("Open goes where the subject lives: a Session to its own view, a pull request to the side panel", () => {
  const given = hosts();
  const acts = actsOf(draft(), given);
  acts.open(subject("session"));
  acts.open(subject("pr"));
  expect(given.onOpenSession.mock.calls).toEqual([["x1"]]);
  expect(opened).toHaveBeenCalledOnce();
  expect(opened).toHaveBeenCalledWith(pr);
});
