// What a move leaves on a Session, read from the ledger: where it went, where it came from, whether it has moved, and
// what Fleet's refusal of one says.

import { describe, expect, test } from "vitest";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { IN_A_TERMINAL, moveRefusalWords, movedFrom, movedRowsOf, movedTo } from "./session-moved";

const repo = (id: string): RepositorySummary => ({
  root: `/code/${id}`,
  records_root: `/records/${id}`,
  manifest: { id, repository: id, path: `${id}/armada.yml`, records_root: `/records/${id}`, version: 1, checks: [] },
});
const REPOSITORIES = [repo("shop"), repo("armada")];

const session = (attachments: Session["attachments"]): Session => ({ id: "s", turn: { state: "idle" }, rows: [], attachments });

const OLD = session([
  { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
  { kind: "forked_to", id: "new", moved: "armada" },
]);
const NEW = session([{ kind: "forked_from", id: "old", moved: "shop", branch: "fix/pin-store-clock" }]);

describe("has moved", () => {
  test("a forked_to that names a repository is a move; a fork is not", () => {
    expect(movedTo(OLD)).toEqual({ kind: "forked_to", id: "new", moved: "armada" });
    expect(movedTo(session([{ kind: "forked_to", id: "new" }]))).toBeUndefined();
    expect(movedTo(NEW)).toBeUndefined();
  });

  test("a forked_from that names a repository is where it came from; a fork is not", () => {
    expect(movedFrom(NEW)?.moved).toBe("shop");
    expect(movedFrom(session([{ kind: "forked_from", id: "old", at: "m2" }]))).toBeUndefined();
    expect(movedFrom(OLD)).toBeUndefined();
  });
});

describe("the moved rows", () => {
  test("the old thread ends with where it went, and nothing starts it", () => {
    const { head, tail } = movedRowsOf(OLD, REPOSITORIES);
    expect(head).toEqual([]);
    expect(tail).toEqual([{ id: "moved-to-new", at: "", kind: "moved", way: "to", session: "new", repository: "armada" }]);
  });

  test("the new thread starts with where it came from, and nothing ends it", () => {
    const { head, tail } = movedRowsOf(NEW, REPOSITORIES);
    expect(head).toEqual([{ id: "moved-from-old", at: "", kind: "moved", way: "from", session: "old", repository: "shop" }]);
    expect(tail).toEqual([]);
  });

  test("a fork has neither row", () => {
    const forked = session([{ kind: "forked_to", id: "a" }, { kind: "forked_from", id: "b", at: "m1" }]);
    expect(movedRowsOf(forked, REPOSITORIES)).toEqual({ head: [], tail: [] });
  });

  test("a repository is named as the picker names it", () => {
    const named = [{ ...repo("shop"), manifest: { ...repo("shop").manifest!, repository: "Shop front" } }];
    expect(movedRowsOf(session([{ kind: "forked_to", id: "n", moved: "shop" }]), named).tail[0]).toMatchObject({ repository: "Shop front" });
  });
});

describe("a refused move", () => {
  const refused = (code: string) => ({ ok: false as const, why: "refused" as const, error: { code, message: "Fleet's words" } }) as never;

  test("Fleet's codes say what the toast says", () => {
    expect(moveRefusalWords(refused("fleet.session_move_same"))).toBe("Already in that repository");
    expect(moveRefusalWords(refused("fleet.session_move_listening"))).toBe(IN_A_TERMINAL);
    expect(moveRefusalWords(refused("fleet.no_such_session"))).toBe("That Job is gone");
    expect(moveRefusalWords(refused("fleet.no_such_manifest"))).toBe("That repository is not served");
  });

  test("any other refusal, and a call that never reached Fleet, is said in the usual words", () => {
    expect(moveRefusalWords(refused("fleet.something_else"))).toBeUndefined();
    expect(moveRefusalWords({ ok: false, why: "not_connected" } as never)).toBeUndefined();
  });
});
