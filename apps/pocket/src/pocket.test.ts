import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { homeOf, type Mac } from "./fleet";
import type { PhoneSession } from "./gateway";
import { codeFromInput, linkOf } from "./pairing";
import { routeOf } from "./router";
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
    expect(await signedMessage("GET", "/api/sessions?state=ended", 1760000000, "")).toBe(`GET/api/sessions?state=ended1760000000${EMPTY}`);
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
    expect(parser.feed('data: {"change":"ses')).toEqual([]);
    expect(parser.feed('sion"}\n')).toEqual([]);
    expect(parser.feed("\ndata: {\"change\":\"resync\"}\n\n")).toEqual(['{"change":"session"}', '{"change":"resync"}']);
  });

  it("joins multi-line data, ignores other fields and comments, and reads CRLF", () => {
    const parser = new SseParser();
    expect(parser.feed(": hello\r\nevent: x\r\ndata: a\r\ndata: b\r\n\r\n")).toEqual(["a\nb"]);
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

describe("linkOf", () => {
  const code = "0123456789abcdef0123456789abcdef";
  it("takes the Mac from the link and this page's Mac for a bare code", () => {
    expect(linkOf(`https://laptop.tail0.ts.net/pair?code=${code}`, "https://studio.tail0.ts.net")).toEqual({ origin: "https://laptop.tail0.ts.net", code });
    expect(linkOf(code, "https://studio.tail0.ts.net")).toEqual({ origin: "https://studio.tail0.ts.net", code });
    expect(linkOf("https://laptop.tail0.ts.net/pair", "https://studio.tail0.ts.net")).toBeUndefined();
  });
});

describe("routeOf", () => {
  it("reads a Job's Mac and id, and what it showed", () => {
    const mac = encodeURIComponent("https://studio.tail0.ts.net");
    expect(routeOf(`/jobs/${mac}/s1`)).toEqual({ screen: "job", origin: "https://studio.tail0.ts.net", id: "s1" });
    expect(routeOf("/jobs/_/s1")).toEqual({ screen: "job", origin: undefined, id: "s1" });
    expect(routeOf(`/jobs/${mac}/s1/file?path=%2Fa%2Fb.md`)).toEqual({ screen: "file", origin: "https://studio.tail0.ts.net", id: "s1", path: "/a/b.md" });
    expect(routeOf(`/jobs/${mac}/s1/sketch?id=sketch%3AHome`)).toEqual({ screen: "sketch", origin: "https://studio.tail0.ts.net", id: "s1", sketch: "sketch:Home" });
    expect(routeOf("/pair?code=abc")).toEqual({ screen: "pair", code: "abc" });
    expect(routeOf("/nowhere")).toEqual({ screen: "home" });
  });
});

describe("homeOf", () => {
  const session = (id: string, extra: Partial<PhoneSession>): PhoneSession => ({ id, kind: "hosted", waiting: false, working: false, held: false, started_at: "2026-10-08T10:00:00Z", ...extra });
  const mac = (origin: string, extra: Partial<Mac>): Mac => ({ machine: { origin, name: origin.slice(8), deviceId: "d", key: {} as CryptoKey }, paired: true, needs: [], live: [], ...extra });
  it("leaves out an unpaired Mac's Jobs and lists it as troubled", () => {
    const home = homeOf({ macs: [mac("https://a", { needs: [session("a1", { waiting: true, waiting_since: "2026-10-08T11:00:00Z" })] }), mac("https://b", { paired: false, needs: [session("b1", { waiting: true })] })] });
    expect(home.needs.map((one) => one.session.id)).toEqual(["a1"]);
    expect(home.troubled.map((one) => one.machine.origin)).toEqual(["https://b"]);
    expect(home.several).toBe(true);
  });
  it("keeps a waiting Job out of Running", () => {
    const home = homeOf({ macs: [mac("https://a", { live: [session("a1", { working: true, waiting: true }), session("a2", { working: true })] })] });
    expect(home.running.map((one) => one.session.id)).toEqual(["a2"]);
    expect(home.several).toBe(false);
  });
  it("lists an open Job with no turn running, which is neither waiting nor ended", () => {
    const home = homeOf({ macs: [mac("https://a", { live: [session("a1", { working: true }), session("a2", {}), session("a3", { waiting: true })] })] });
    expect(home.running.map((one) => one.session.id).sort()).toEqual(["a1", "a2"]);
  });
});

describe("the source folder", () => {
  // Vite tries `.ts` before `.tsx`, so on a Mac `./Pair` finds pair.ts and the build breaks.
  it("has no two files that differ only in case or extension", () => {
    const stems = readdirSync(new URL(".", import.meta.url)).map((name) => name.replace(/\.[^.]+$/, "").toLowerCase());
    expect(stems.filter((stem, at) => stems.indexOf(stem) !== at)).toEqual([]);
  });
});

describe("the text fields", () => {
  // happy-dom does not compute imported CSS, so the stylesheet is read as text. iOS zooms into a focused field under 16 px.
  const css = readFileSync(new URL("./pocket.css", import.meta.url), "utf8");
  const tokens = readFileSync(new URL("../../../packages/tokens/tokens.css", import.meta.url), "utf8");
  it("takes --text-touch-field, which is 16 px, on every field the phone types in", () => {
    expect(tokens).toMatch(/--text-touch-field:\s*16px/);
    const rule = /main\.pk-app\s*:is\(([^)]*)\)\s*\{([^}]*)\}/.exec(css);
    expect(rule?.[2]).toMatch(/font-size:\s*var\(--text-touch-field\)/);
    for (const field of ["textarea", "input", "select", '[contenteditable="true"]']) expect(rule?.[1]).toContain(field);
  });
});
