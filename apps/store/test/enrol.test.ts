import { afterEach, describe, expect, it, vi } from "vitest";
import { STORE, call, toStore, claim, enrolDevice, everythingStoredFor, forgeAnswers, hash, holder, newKeyPair } from "./support";

afterEach(() => vi.restoreAllMocks());

const enrol = (body: unknown) =>
  toStore(`${STORE}/v1/enrol`, { method: "POST", body: JSON.stringify(body) });

describe("enrolling a device", () => {
  it("files it under the forge's numeric id and keeps no token", async () => {
    const token = "device-flow-token-for-this-test";
    const { spki } = await newKeyPair();
    const logs = (["log", "info", "warn", "error", "debug"] as const).map((level) => vi.spyOn(console, level));
    const forge = forgeAnswers(424242);

    const answer = await enrol({ token, public_key: spki, machine: "studio" });
    const text = await answer.text();

    expect(answer.status).toBe(200);
    const { device } = JSON.parse(text) as { device: string };
    expect(device).toMatch(/^424242\.[0-9a-f]{32}$/);
    expect(text).not.toContain(token);

    expect(forge).toHaveBeenCalledTimes(1);
    const [url, init] = forge.mock.calls[0]!;
    expect(String(url)).toBe("https://forge.test/user");
    expect((init!.headers as Record<string, string>).Authorization).toBe(`Bearer ${token}`);

    const stored = await everythingStoredFor(424242);
    expect(stored).toContain(device);
    expect(stored).toContain(spki);
    expect(stored).not.toContain(token);
    expect(stored).not.toContain("someone");
    expect(stored).not.toContain("A Name");

    for (const log of logs) expect(JSON.stringify(log.mock.calls)).not.toContain(token);
  });

  it("puts two machines of one account in one store, and another account's apart", async () => {
    const [a, b, other] = [await enrolDevice(7001), await enrolDevice(7001), await enrolDevice(7002)];
    expect(a.id.startsWith("7001.")).toBe(true);
    expect(other.id.startsWith("7002.")).toBe(true);

    await claim(a, hash("a"), hash("b"), holder("j-1"));
    expect((await call(b, "GET", `/v1/claims/${hash("a")}/${hash("b")}`)).status).toBe(200);
    expect((await call(other, "GET", `/v1/claims/${hash("a")}/${hash("b")}`)).status).toBe(404);
  });

  it("registers nothing when the forge refuses the token", async () => {
    const { spki } = await newKeyPair();
    forgeAnswers(7100, 401);

    const answer = await enrol({ token: "expired-token", public_key: spki, machine: "studio" });

    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "forge_refused" });
    expect(await everythingStoredFor(7100)).not.toContain(spki);
  });

  it("says so when the forge cannot be reached, and registers nothing", async () => {
    const { spki } = await newKeyPair();
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("offline"));

    const answer = await enrol({ token: "any-token", public_key: spki, machine: "studio" });

    expect(answer.status).toBe(502);
    expect(await answer.json()).toEqual({ error: "forge_unreachable" });
  });

  it("refuses a key that is not a P-256 public key before it asks the forge", async () => {
    const forge = forgeAnswers(7200);
    const answer = await enrol({ token: "any-token", public_key: "bm90IGEga2V5", machine: "studio" });
    expect(answer.status).toBe(400);
    expect(forge).not.toHaveBeenCalled();
  });

  it("refuses a body with a field it does not know", async () => {
    const { spki } = await newKeyPair();
    const forge = forgeAnswers(7300);
    const answer = await enrol({ token: "any-token", public_key: spki, machine: "studio", login: "someone" });
    expect(answer.status).toBe(400);
    expect(forge).not.toHaveBeenCalled();
  });

  it("refuses an id that is not a positive number", async () => {
    const { spki } = await newKeyPair();
    for (const id of ["12", 0, -4, 1.5]) {
      vi.spyOn(globalThis, "fetch").mockImplementation(async () => Response.json({ id }));
      const answer = await enrol({ token: "any-token", public_key: spki, machine: "studio" });
      expect(answer.status).toBe(502);
      vi.restoreAllMocks();
    }
  });
});
