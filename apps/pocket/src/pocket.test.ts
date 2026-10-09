import { describe, expect, it, vi } from "vitest";

import { ago, toPocket } from "./data";
import type { PhoneJob } from "./gateway";
import { codeFromInput } from "./pair";
import { bytesOf } from "./push";
import { SseParser } from "./sse";
import { bodyHash, hex, signHeaders, signedMessage } from "./signing";

// A fixed P-256 key (test only).
const PRIVATE = {
  kty: "EC", crv: "P-256",
  x: "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
  y: "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
  d: "jpsQnnGQmL-YBIffH1136cspYG6-0iY7X1fCE9-E9LI",
};

const EMPTY = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

describe("the signer", () => {
  it("joins method, target, time and the body hash with nothing between", async () => {
    expect(await bodyHash("")).toBe(EMPTY);
    expect(await signedMessage("GET", "/api/jobs?state=done", 1760000000, "")).toBe(`GET/api/jobs?state=done1760000000${EMPTY}`);
  });

  it("signs that message with the key, as 64 raw bytes the public key verifies", async () => {
    const key = await crypto.subtle.importKey("jwk", PRIVATE, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
    const headers = await signHeaders(key, "abc", "POST", "/api/x", '{"a":1}', 1760000000);
    expect(headers["X-Pocket-Device"]).toBe("abc");
    expect(headers["X-Pocket-Time"]).toBe("1760000000");
    expect(headers["X-Pocket-Signature"]).toMatch(/^[0-9a-f]{128}$/);
    const { d: _private, ...open } = PRIVATE;
    const verify = await crypto.subtle.importKey("jwk", open, { name: "ECDSA", namedCurve: "P-256" }, false, ["verify"]);
    const message = new TextEncoder().encode(await signedMessage("POST", "/api/x", 1760000000, '{"a":1}'));
    const raw = Uint8Array.from(headers["X-Pocket-Signature"].match(/../g)!.map((byte) => parseInt(byte, 16)));
    expect(await crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, verify, raw, message)).toBe(true);
    expect(hex(raw)).toBe(headers["X-Pocket-Signature"]);
  });
});

describe("the SSE parser", () => {
  it("returns an event once its blank line arrives, across chunk boundaries", () => {
    const parser = new SseParser();
    expect(parser.feed('data: {"change":"cre')).toEqual([]);
    expect(parser.feed('ated"}\n')).toEqual([]);
    expect(parser.feed("\ndata: {\"change\":\"resync\"}\n\n")).toEqual(['{"change":"created"}', '{"change":"resync"}']);
  });

  it("joins multi-line data, ignores other fields and comments, and reads CRLF", () => {
    const parser = new SseParser();
    expect(parser.feed(": hello\r\nevent: x\r\ndata: a\r\ndata: b\r\n\r\n")).toEqual(["a\nb"]);
  });
});

const NOW = Date.parse("2026-10-08T12:00:00Z");
const base: PhoneJob = { id: "j1", title: "T", status: "escalated", asking: false, created_at: "2026-10-08T09:50:00Z" };

describe("data.ts mapping", () => {
  it("counts ages the way the rows draw them", () => {
    expect(ago("2026-10-08T11:59:30Z", NOW)).toBe("now");
    expect(ago("2026-10-08T11:46:00Z", NOW)).toBe("14m");
    expect(ago("2026-10-08T11:00:00Z", NOW)).toBe("1h");
    expect(ago("2026-10-08T09:50:00Z", NOW)).toBe("2h 10m");
  });

  it("maps a needs row", () => {
    const job = toPocket({ ...base, repository: "armada", reason: "stalled", waiting_since: "2026-10-08T11:19:00Z", step: { at: 2, of: 5, name: "Implement" }, verdict: "veto", checks: [{ name: "typecheck", passed: true }], pull_request: "https://example.test/armada/pull/2003" }, NOW);
    expect(job).toMatchObject({
      repository: "armada", reason: "stalled", age: "2h 10m", quiet: "41m",
      step: { at: 2, of: 5, name: "Implement" }, verdict: { says: "veto", line: "Veto" },
      checks: [{ name: "typecheck", passed: true }], pr: { number: 2003, url: "example.test/armada/pull/2003" },
    });
  });

  it("counts a finished Job's age from when it ended, and leaves absent fields absent", () => {
    const job = toPocket({ ...base, status: "completed_success", ended_at: "2026-10-08T11:00:00Z" }, NOW);
    expect(job.age).toBe("1h");
    expect(job).toMatchObject({ repository: "", quiet: undefined, verdict: undefined, pr: undefined });
  });
});

describe("the push key", () => {
  it("decodes base64url, without padding, to the bytes of the point", () => {
    expect(Array.from(bytesOf("BAH_-w"))).toEqual([4, 1, 255, 251]);
  });
});

describe("codeFromInput", () => {
  const code = "0123456789abcdef0123456789abcdef";
  it("reads the code from a whole link", () => {
    expect(codeFromInput(`https://mac.example:7777/pair?code=${code}`)).toBe(code);
    expect(codeFromInput(`  https://mac.example/pair?code=${code}\n`)).toBe(code);
  });
  it("accepts a bare code", () => expect(codeFromInput(code)).toBe(code));
  it("refuses junk", () => {
    expect(codeFromInput("")).toBe("");
    expect(codeFromInput("hello")).toBe("");
    expect(codeFromInput("https://mac.example/pair?code=abc")).toBe("");
  });
});

describe("the acts", () => {
  type Sent = { url: string; method: string; body: string; signed: boolean };
  const sent: Sent[] = [];
  let reply: () => Response = () => new Response(null, { status: 204 });

  const world = async () => {
    const key = await crypto.subtle.importKey("jwk", PRIVATE, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
    vi.resetModules();
    sent.length = 0;
    vi.doMock("./device", () => ({ paired: async () => ({ key, deviceId: "abc" }) }));
    vi.doMock("./router", () => ({ go: () => undefined }));
    vi.stubGlobal("fetch", async (url: string, init: { method: string; body?: string; headers: Record<string, string> }) => {
      sent.push({ url, method: init.method, body: init.body ?? "", signed: /^[0-9a-f]{128}$/.test(init.headers["X-Pocket-Signature"] ?? "") });
      return url.startsWith("/api/needs") ? new Response(JSON.stringify({ needs: [], sessions: [] })) : url.startsWith("/api/jobs?") ? new Response(JSON.stringify({ jobs: [] })) : reply();
    });
    return import("./data");
  };
  const acts = () => sent.filter((one) => one.method === "POST");

  it("posts each Job act, signed, with the body the route reads", async () => {
    const data = await world();
    await data.actOn("j1", "approve");
    await data.actOn("j1", "restart_step");
    await data.actOn("j1", "kill");
    await data.actOn("j1", "redispatch");
    await data.actOn("j1", "approve_review");
    await data.redirect("j1", "use the cache");
    await data.requestChanges("j1", "too broad");
    expect(acts().map(({ url, body, signed }) => [url, body, signed])).toEqual([
      ["/api/jobs/j1/approve", "{}", true],
      ["/api/jobs/j1/restart_step", "{}", true],
      ["/api/jobs/j1/kill", "{}", true],
      ["/api/jobs/j1/redispatch", "{}", true],
      ["/api/jobs/j1/approve_review", "{}", true],
      ["/api/jobs/j1/redirect", '{"text":"use the cache"}', true],
      ["/api/jobs/j1/request_changes", '{"reason":"too broad"}', true],
    ]);
  });

  it("answers a permission ask with the decision and a question ask with the choices", async () => {
    const data = await world();
    await data.answer({ session_id: "s1", ask_id: "a1", answer: "allow_once" });
    await data.answer({ session_id: "s1", ask_id: "a2", answer: [{ question: "Close now?", chosen: ["Now"] }, { question: "Why?", chosen: ["my own words"] }] });
    expect(acts().map(({ url, body }) => [url, JSON.parse(body)])).toEqual([
      ["/api/sessions/answer", { session_id: "s1", ask_id: "a1", answer: "allow_once" }],
      ["/api/sessions/answer", { session_id: "s1", ask_id: "a2", answer: [{ question: "Close now?", chosen: ["Now"] }, { question: "Why?", chosen: ["my own words"] }] }],
    ]);
  });

  it("dispatches the line to the repository", async () => {
    const data = await world();
    reply = () => new Response(JSON.stringify({ jobs: [] }), { status: 201 });
    await data.dispatch({ text: "Trim the pool", repository: "armada" });
    expect(acts().map(({ url, body, signed }) => [url, JSON.parse(body), signed])).toEqual([["/api/jobs", { text: "Trim the pool", repository: "armada" }, true]]);
    reply = () => new Response(null, { status: 204 });
  });

  it("keeps the draft and answers the Gateway's sentence when the act is refused", async () => {
    const data = await world();
    reply = () => new Response("That Job is not waiting.", { status: 409 });
    data.DRAFTS.set("redirect-j1", "use the cache");
    expect(await data.attempt(() => data.redirect("j1", "use the cache"), ["redirect-j1"])).toBe("That Job is not waiting.");
    expect(data.DRAFTS.get("redirect-j1")).toBe("use the cache");
    reply = () => new Response(null, { status: 204 });
    expect(await data.attempt(() => data.redirect("j1", "use the cache"), ["redirect-j1"])).toBeNull();
    expect(data.DRAFTS.has("redirect-j1")).toBe(false);
  });
});
