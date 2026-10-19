import { describe, expect, it } from "vitest";
import { signRequest } from "../src/signing";
import { STORE, call, toStore, claim, enrolDevice, everythingStoredFor, fact, hash, holder, newKeyPair } from "./support";

const now = () => Math.floor(Date.now() / 1000);

describe("a claim", () => {
  it("is won by exactly one of eight concurrent claims", async () => {
    const [a, b] = [await enrolDevice(5001), await enrolDevice(5001)];
    const attempts = Array.from({ length: 8 }, (_, n) => claim(n % 2 ? a : b, hash("a"), hash("b"), holder(`j-${n}`)));
    const answers = await Promise.all(attempts);

    const won = answers.filter((answer) => answer.status === 200);
    const lost = answers.filter((answer) => answer.status === 409);
    expect(won).toHaveLength(1);
    expect(lost).toHaveLength(7);

    const winner = ((await won[0]!.json()) as { holder: unknown }).holder;
    for (const answer of lost) expect(((await answer.json()) as { holder: unknown }).holder).toEqual(winner);
    const read = await call(a, "GET", `/v1/claims/${hash("a")}/${hash("b")}`);
    expect(((await read.json()) as { holder: unknown }).holder).toEqual(winner);
  });

  it("tells the second machine who holds it", async () => {
    const [a, b] = [await enrolDevice(5002), await enrolDevice(5002)];
    expect((await claim(a, hash("a"), hash("b"), holder("j-1", "laptop"))).status).toBe(200);
    const second = await claim(b, hash("a"), hash("b"), holder("s-9", "desktop"));
    expect(second.status).toBe(409);
    expect(await second.json()).toEqual({ holder: holder("j-1", "laptop") });
  });

  it("leaves another red of the same repository free", async () => {
    const a = await enrolDevice(5003);
    expect((await claim(a, hash("a"), hash("b"), holder("j-1"))).status).toBe(200);
    expect((await claim(a, hash("a"), hash("c"), holder("j-2"))).status).toBe(200);
  });

  it("reads as 404 until taken, and 404 again once given back", async () => {
    const a = await enrolDevice(5004);
    const path = `/v1/claims/${hash("a")}/${hash("b")}`;
    expect((await call(a, "GET", path)).status).toBe(404);
    await claim(a, hash("a"), hash("b"), holder("j-1"));
    expect((await call(a, "GET", path)).status).toBe(200);
    expect((await call(a, "DELETE", path)).status).toBe(204);
    expect((await call(a, "GET", path)).status).toBe(404);
  });

  it("can be taken over by deleting it and claiming again", async () => {
    const [a, b] = [await enrolDevice(5005), await enrolDevice(5005)];
    await claim(a, hash("a"), hash("b"), holder("j-1"));
    await call(b, "DELETE", `/v1/claims/${hash("a")}/${hash("b")}`);
    expect((await claim(b, hash("a"), hash("b"), holder("j-2"))).status).toBe(200);
  });

  it("is cleared for a whole repository and no other", async () => {
    const a = await enrolDevice(5006);
    await claim(a, hash("a"), hash("b"), holder("j-1"));
    await claim(a, hash("a"), hash("c"), holder("j-2"));
    await claim(a, hash("d"), hash("b"), holder("j-3"));
    expect((await call(a, "DELETE", `/v1/claims/${hash("a")}`)).status).toBe(204);
    expect((await call(a, "GET", `/v1/claims/${hash("a")}/${hash("b")}`)).status).toBe(404);
    expect((await call(a, "GET", `/v1/claims/${hash("a")}/${hash("c")}`)).status).toBe(404);
    expect((await call(a, "GET", `/v1/claims/${hash("d")}/${hash("b")}`)).status).toBe(200);
  });

  it("is not seen by another account", async () => {
    const [mine, theirs] = [await enrolDevice(5007), await enrolDevice(5008)];
    await claim(mine, hash("a"), hash("b"), holder("j-1"));
    expect((await call(theirs, "GET", `/v1/claims/${hash("a")}/${hash("b")}`)).status).toBe(404);
    expect((await claim(theirs, hash("a"), hash("b"), holder("j-2"))).status).toBe(200);
  });
});

describe("a row", () => {
  it("refuses a field a holder does not have", async () => {
    const a = await enrolDevice(5101);
    const withTitle = { ...holder("j-1"), title: "Fix the login bug" };
    expect((await claim(a, hash("a"), hash("b"), withTitle as never)).status).toBe(400);
    expect((await call(a, "POST", "/v1/claims", { repo: hash("a"), red: hash("b"), holder: holder("j-1"), name: "x" })).status).toBe(400);
    expect((await call(a, "PUT", `/v1/owners/${hash("c")}`, { holder: withTitle })).status).toBe(400);
  });

  it("refuses a key that is a name and not a hash", async () => {
    const a = await enrolDevice(5102);
    expect((await claim(a, "armada", "main-is-red", holder("j-1"))).status).toBe(400);
    expect((await call(a, "PUT", "/v1/owners/armada-pull-1", { holder: holder("j-1") })).status).toBe(400);
  });

  it("refuses a holder that is not a session or a job", async () => {
    const a = await enrolDevice(5103);
    expect((await claim(a, hash("a"), hash("b"), { ...holder("j-1"), kind: "person" } as never)).status).toBe(400);
    expect((await claim(a, hash("a"), hash("b"), { ...holder("j-1"), since: "yesterday" } as never)).status).toBe(400);
  });
});

describe("an owner", () => {
  it("is put, read by keys, replaced and deleted", async () => {
    const a = await enrolDevice(5201);
    const [one, two, three] = [hash("1"), hash("2"), hash("3")];
    await call(a, "PUT", `/v1/owners/${one}`, { holder: holder("s-1", "laptop") });
    await call(a, "PUT", `/v1/owners/${two}`, { holder: holder("j-2", "desktop") });

    const read = await call(a, "GET", `/v1/owners?keys=${one},${two},${three}`);
    expect(await read.json()).toEqual({ [one]: holder("s-1", "laptop"), [two]: holder("j-2", "desktop") });

    await call(a, "PUT", `/v1/owners/${one}`, { holder: holder("s-7", "desktop") });
    expect(await (await call(a, "GET", `/v1/owners?keys=${one}`)).json()).toEqual({ [one]: holder("s-7", "desktop") });

    expect((await call(a, "DELETE", `/v1/owners/${one}`)).status).toBe(204);
    expect(await (await call(a, "GET", `/v1/owners?keys=${one},${two}`)).json()).toEqual({ [two]: holder("j-2", "desktop") });
  });

  it("is shared by the account's machines", async () => {
    const [a, b] = [await enrolDevice(5202), await enrolDevice(5202)];
    await call(a, "PUT", `/v1/owners/${hash("1")}`, { holder: holder("s-1") });
    expect(await (await call(b, "GET", `/v1/owners?keys=${hash("1")}`)).json()).toEqual({ [hash("1")]: holder("s-1") });
  });
});

describe("a repository's runs", () => {
  const path = `/v1/runs/${hash("r")}`;

  it("are put and read back as they were sent", async () => {
    const a = await enrolDevice(5501);
    const facts = [fact(), { ...fact({ workspace: "apps/web", kind: "dev_server", command: "pnpm dev" }), port: 5173 }];
    expect((await call(a, "PUT", path, { facts })).status).toBe(204);
    const read = await call(a, "GET", path);
    expect(read.status).toBe(200);
    expect(await read.json()).toEqual({ facts });
  });

  it("are replaced whole by the next put, not merged", async () => {
    const a = await enrolDevice(5502);
    await call(a, "PUT", path, { facts: [fact(), fact({ command: "cargo clippy", kind: "lint" })] });
    const later = [fact({ command: "cargo build", kind: "build" })];
    expect((await call(a, "PUT", path, { facts: later })).status).toBe(204);
    expect(await (await call(a, "GET", path)).json()).toEqual({ facts: later });
  });

  it("are shared by the account's machines and by no other account", async () => {
    const [a, b, other] = [await enrolDevice(5503), await enrolDevice(5503), await enrolDevice(5504)];
    await call(a, "PUT", path, { facts: [fact()] });
    expect(await (await call(b, "GET", path)).json()).toEqual({ facts: [fact()] });
    const theirs = await call(other, "GET", path);
    expect(theirs.status).toBe(404);
    expect(await theirs.json()).toEqual({ error: "no_runs" });
  });

  it("are removed by a put of an empty list, which holds no row", async () => {
    const a = await enrolDevice(5505);
    await call(a, "PUT", path, { facts: [fact()] });
    expect((await call(a, "PUT", path, { facts: [] })).status).toBe(204);
    const read = await call(a, "GET", path);
    expect(read.status).toBe(404);
    expect(await read.json()).toEqual({ error: "no_runs" });
    expect(JSON.parse(await everythingStoredFor(5505)).find((table: { name: string }) => table.name === "runs").rows).toEqual([]);
  });

  it("read as 404 no_runs until put, and an empty put for a new repository writes nothing", async () => {
    const a = await enrolDevice(5506);
    expect(await (await call(a, "GET", path)).json()).toEqual({ error: "no_runs" });
    expect((await call(a, "PUT", path, { facts: [] })).status).toBe(204);
    expect((await call(a, "GET", path)).status).toBe(404);
  });

  it("keep a repository's facts apart from another's", async () => {
    const a = await enrolDevice(5507);
    await call(a, "PUT", `/v1/runs/${hash("r")}`, { facts: [fact()] });
    await call(a, "PUT", `/v1/runs/${hash("s")}`, { facts: [fact({ command: "make" })] });
    expect(await (await call(a, "GET", `/v1/runs/${hash("r")}`)).json()).toEqual({ facts: [fact()] });
  });

  describe("refuse with 400 invalid_body", () => {
    const refused = async (id: number, body: unknown) => {
      const a = await enrolDevice(id);
      const answer = await call(a, "PUT", path, body);
      expect(answer.status).toBe(400);
      expect(await answer.json()).toEqual({ error: "invalid_body" });
      expect((await call(a, "GET", path)).status).toBe(404);
    };

    it("a fact with a field it does not have", () => refused(5601, { facts: [{ ...fact(), title: "x" }] }));
    it("a body with a field beside facts", () => refused(5602, { facts: [fact()], name: "armada" }));
    it("a body whose facts are missing", () => refused(5603, {}));
    it.each(["workspace", "kind", "command", "succeeded", "last_ran", "source"])("a fact with no %s", (field) => {
      const { [field]: _gone, ...rest } = fact() as Record<string, unknown>;
      return refused(5610, { facts: [rest] });
    });
    it.each(["jobs", "", 7])("a kind of %j", (kind) => refused(5604, { facts: [fact({ kind: kind as never })] }));
    it.each([0, 65536, 1.5, "80", -1, null])("a port of %j", (port) => refused(5605, { facts: [{ ...fact(), port }] }));
    it("two facts of one workspace, kind and command", () =>
      refused(5606, { facts: [fact(), fact({ succeeded: false, last_ran: 9 })] }));
    it("201 facts", () =>
      refused(5607, { facts: Array.from({ length: 201 }, (_, n) => fact({ command: `cmd ${n}` })) }));
    it.each([
      ["a workspace that is empty", { workspace: "" }],
      ["a workspace that is absolute", { workspace: "/etc" }],
      ["a workspace that leaves the repository", { workspace: "../x" }],
      ["a workspace of 201 characters", { workspace: "w".repeat(201) }],
      ["a command with a newline", { command: "pnpm test\nrm -rf x" }],
      ["a command of 401 characters", { command: "c".repeat(401) }],
      ["a command that is empty", { command: "" }],
      ["a source with a space", { source: "my jobs" }],
      ["a source of 33 characters", { source: "s".repeat(33) }],
      ["a last_ran that is a fraction", { last_ran: 1.5 }],
      ["a last_ran that is negative", { last_ran: -1 }],
      ["a succeeded that is text", { succeeded: "yes" }],
    ])("%s", (_name, change) => refused(5608, { facts: [fact(change as never)] }));
    it("facts that are not a list", () => refused(5609, { facts: fact() }));
    it("a body that is not an object", () => refused(5611, [fact()]));
  });

  it("accept the limits themselves: 200 facts, a 200-character workspace, a 400-character command, port 65535", async () => {
    const a = await enrolDevice(5612);
    const long = fact({ workspace: "w".repeat(200), command: "c".repeat(400), port: 65535 });
    const many = Array.from({ length: 199 }, (_, n) => fact({ command: `cmd ${n}` }));
    const facts = [long, ...many];
    expect((await call(a, "PUT", path, { facts })).status).toBe(204);
    expect(await (await call(a, "GET", path)).json()).toEqual({ facts });
  });

  it("refuse a key that is a name and not a hash", async () => {
    const a = await enrolDevice(5613);
    expect((await call(a, "PUT", "/v1/runs/armada", { facts: [fact()] })).status).toBe(400);
    expect((await call(a, "GET", "/v1/runs/armada")).status).toBe(400);
  });

  it("refuse a method the path does not take", async () => {
    const a = await enrolDevice(5614);
    expect((await call(a, "DELETE", path)).status).toBe(405);
  });

  it("are refused unsigned, and from a device that never enrolled", async () => {
    const a = await enrolDevice(5615);
    await call(a, "PUT", path, { facts: [fact()] });
    const bare = await toStore(`${STORE}${path}`);
    expect(bare.status).toBe(401);
    expect(await bare.json()).toEqual({ error: "unsigned" });
    const stranger = { ...a, id: `5615.${"e".repeat(32)}`, key: (await newKeyPair()).key };
    const answer = await call(stranger, "PUT", path, { facts: [fact()] });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "unknown_device" });
  });
});

describe("a device", () => {
  it("stops being accepted once signed out", async () => {
    const [a, b] = [await enrolDevice(5301), await enrolDevice(5301)];
    expect((await call(a, "DELETE", `/v1/devices/${b.id}`)).status).toBe(204);
    const after = await call(b, "GET", `/v1/claims/${hash("a")}/${hash("b")}`);
    expect(after.status).toBe(401);
    expect(await after.json()).toEqual({ error: "unknown_device" });
  });
});

describe("a request", () => {
  const target = `/v1/claims/${hash("a")}/${hash("b")}`;

  it("signed by a key that is not the device's is refused", async () => {
    const a = await enrolDevice(5401);
    const stranger = await newKeyPair();
    const answer = await call(a, "GET", target, undefined, { key: stranger.key });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "bad_signature" });
  });

  it("whose body changed after signing is refused", async () => {
    const a = await enrolDevice(5402);
    const body = { repo: hash("a"), red: hash("b"), holder: holder("j-1") };
    const answer = await call(a, "POST", "/v1/claims", body, { signedBody: JSON.stringify({ ...body, holder: holder("j-2") }) });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "bad_signature" });
    expect((await call(a, "GET", target)).status).toBe(404);
  });

  it("whose path changed after signing is refused", async () => {
    const a = await enrolDevice(5403);
    const headers = await signRequest(a.key, a.id, "DELETE", `/v1/claims/${hash("a")}`);
    const answer = await toStore(`${STORE}/v1/claims/${hash("d")}`, { method: "DELETE", headers });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "bad_signature" });
  });

  it("older than the skew is refused, and so is one from the future", async () => {
    const a = await enrolDevice(5404);
    for (const time of [now() - 120, now() + 120]) {
      const answer = await call(a, "GET", target, undefined, { time: String(time) });
      expect(answer.status).toBe(401);
      expect(await answer.json()).toEqual({ error: "stale_time" });
    }
  });

  it("inside the skew is accepted", async () => {
    const a = await enrolDevice(5405);
    for (const time of [now() - 30, now() + 30]) {
      expect((await call(a, "GET", target, undefined, { time: String(time) })).status).toBe(404);
    }
  });

  it("replayed with the same nonce is refused", async () => {
    const a = await enrolDevice(5406);
    const headers = await signRequest(a.key, a.id, "GET", target);
    const send = () => toStore(`${STORE}${target}`, { headers });

    expect((await send()).status).toBe(404);
    const replay = await send();
    expect(replay.status).toBe(401);
    expect(await replay.json()).toEqual({ error: "replayed_nonce" });
  });

  it("replayed twice at once lets one through", async () => {
    const a = await enrolDevice(5407);
    const headers = await signRequest(a.key, a.id, "GET", target);
    const answers = await Promise.all([1, 2, 3, 4].map(() => toStore(`${STORE}${target}`, { headers })));
    expect(answers.map((answer) => answer.status).sort()).toEqual([401, 401, 401, 404]);
  });

  it("a nonce is the device's own, so another device may use the same text", async () => {
    const [a, b] = [await enrolDevice(5408), await enrolDevice(5408)];
    const nonce = "0123456789abcdef0123456789abcdef";
    expect((await call(a, "GET", target, undefined, { nonce })).status).toBe(404);
    expect((await call(b, "GET", target, undefined, { nonce })).status).toBe(404);
  });

  it("with no signature is refused", async () => {
    const a = await enrolDevice(5409);
    const answer = await call(a, "GET", target, undefined, { headers: { "X-Armada-Signature": "" } });
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "unsigned" });
    expect((await toStore(`${STORE}${target}`)).status).toBe(401);
  });

  it("from a device that never enrolled is refused", async () => {
    const stranger = { id: `5410.${"0".repeat(32)}`, key: (await newKeyPair()).key, account: 5410, token: "" };
    const answer = await call(stranger, "GET", target);
    expect(answer.status).toBe(401);
    expect(await answer.json()).toEqual({ error: "unknown_device" });
  });
});
