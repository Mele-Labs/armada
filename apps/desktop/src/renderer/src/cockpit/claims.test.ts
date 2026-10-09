import { beforeEach, describe, expect, test } from "vitest";

import { attachPr, claimBody, claims, forgetClaims, wasRefused } from "./claims";

const PULL = { branch: "chore/bump-lockfile", url: "https://git.example/pull/1823", open: true };
const session = { kind: "session", id: "s10", title: "Migration notes" } as const;
const job = { kind: "job", id: "j1", title: "Cap the log reader" } as const;

beforeEach(forgetClaims);

describe("claiming a pull request nobody holds", () => {
  test("the body carries exactly one of session_id and job_id", () => {
    expect(claimBody(1823, session)).toEqual({ number: 1823, session_id: "s10" });
    expect(claimBody(1823, job)).toEqual({ number: 1823, job_id: "j1" });
  });

  test("is answered with the pull request and who holds it", async () => {
    const answer = await attachPr(1823, session, PULL);
    expect(answer).toEqual({ number: 1823, branch: PULL.branch, url: PULL.url, holder_kind: "session", holder_id: "s10" });
    expect(claims().get(1823)).toEqual(session);
  });

  test("is refused, naming the holder, where one already holds it", async () => {
    await attachPr(1823, session, PULL);
    const refused = await attachPr(1823, job, PULL);
    expect(wasRefused(refused)).toBe(true);
    expect(refused).toMatchObject({ why: "held", holder: "Migration notes" });
    expect((refused as { said: string }).said).toContain("Migration notes");
    expect(claims().get(1823)).toEqual(session);
  });

  test("is refused where the pull request is not open", async () => {
    const refused = await attachPr(1823, session, { ...PULL, open: false });
    expect(refused).toMatchObject({ refused: true, why: "not_open" });
    expect(claims().size).toBe(0);
  });
});
