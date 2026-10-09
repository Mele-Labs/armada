// What each route's answer, and each failure, makes of Settings → Phone's state.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { PhoneAnswer, PhoneRequest } from "./api";
import { createPhoneGatewaySource } from "./phone-gateway-source";

const ok = (body: unknown): PhoneAnswer => ({ ok: true, body });
const refused = (said: string): PhoneAnswer => ({ ok: false, why: "refused", said });
const DOWN: PhoneAnswer = { ok: false, why: "unreachable" };
const DEVICES = [{ id: "d1", name: "Nick's iPhone", created_at: 100, last_seen_at: 400 }, { id: "d2", name: "iPad", created_at: 200, last_seen_at: null }];

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

/** A source answering from `answers` by operation, with every request kept. */
function over(answers: Partial<Record<PhoneRequest["op"], PhoneAnswer>>) {
  const asked: PhoneRequest[] = [];
  const source = createPhoneGatewaySource(async (request) => {
    asked.push(request);
    return answers[request.op] ?? ok(null);
  });
  const off = source.subscribe(() => undefined);
  return { source, asked, off };
}

test("listening reads the status and the devices, last seen in ms and creation where never seen", async () => {
  const { source } = over({ status: ok(null), devices: ok(DEVICES) });
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().gateway).toEqual({ state: "running" });
  expect(source.get().phones).toEqual([
    { id: "d1", name: "Nick's iPhone", seenAt: 400_000 },
    { id: "d2", name: "iPad", seenAt: 200_000 },
  ]);
});

test("a refused connection is the Gateway not running", async () => {
  const { source } = over({ status: DOWN });
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().gateway).toEqual({ state: "not_running" });
});

test("any other refusal of the status is the Gateway's sentence as it came", async () => {
  const { source } = over({ status: refused("Tailscale is not signed in on this Mac.") });
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().gateway).toEqual({ state: "said", said: "Tailscale is not signed in on this Mac." });
});

test("pairing shows the address and code, and expires with the code", async () => {
  const { source } = over({ start: ok({ code: "c0de", address: "https://m.ts.net", expires_at: Math.floor(Date.now() / 1000) + 300 }) });
  await vi.advanceTimersByTimeAsync(0);
  source.pair();
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().pairing?.url).toBe("https://m.ts.net/pair?code=c0de");
  await vi.advanceTimersByTimeAsync(301_000);
  expect(source.get().pairing).toBeNull();
});

test("a code is polled every 2 s while shown, and not once it is gone", async () => {
  const { source, asked } = over({
    start: ok({ code: "c0de", address: "https://m.ts.net", expires_at: Math.floor(Date.now() / 1000) + 10 }),
    pending: ok([{ code: "other", name: "Not mine", claimed_at: 1 }, { code: "c0de", name: "Pixel 9", claimed_at: 2 }]),
  });
  await vi.advanceTimersByTimeAsync(0);
  source.pair();
  await vi.advanceTimersByTimeAsync(0);
  expect(asked.filter((one) => one.op === "pending")).toHaveLength(0);
  await vi.advanceTimersByTimeAsync(4000);
  expect(asked.filter((one) => one.op === "pending")).toHaveLength(2);
  expect(source.get().claim).toEqual({ device: "Pixel 9" });
  await vi.advanceTimersByTimeAsync(7000);
  const after = asked.filter((one) => one.op === "pending").length;
  await vi.advanceTimersByTimeAsync(10_000);
  expect(asked.filter((one) => one.op === "pending")).toHaveLength(after);
});

test("confirm sends the code, then reads the devices again", async () => {
  const { source, asked } = over({
    start: ok({ code: "c0de", address: "https://m.ts.net", expires_at: Math.floor(Date.now() / 1000) + 300 }),
    devices: ok(DEVICES),
  });
  await vi.advanceTimersByTimeAsync(0);
  source.pair();
  await vi.advanceTimersByTimeAsync(0);
  source.confirm();
  await vi.advanceTimersByTimeAsync(0);
  expect(asked).toContainEqual({ op: "confirm", code: "c0de" });
  expect(asked.at(-1)).toEqual({ op: "devices" });
  expect(source.get().pairing).toBeNull();
  expect(source.get().claim).toBeNull();
});

test("a refused confirm shows the Gateway's sentence", async () => {
  const { source } = over({
    start: ok({ code: "c0de", address: "https://m.ts.net", expires_at: Math.floor(Date.now() / 1000) + 300 }),
    confirm: refused("No phone has entered this code yet."),
  });
  await vi.advanceTimersByTimeAsync(0);
  source.pair();
  await vi.advanceTimersByTimeAsync(0);
  source.confirm();
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().gateway).toEqual({ state: "said", said: "No phone has entered this code yet." });
});

test("unpair sends the id and reads the devices again; a Gateway gone says so", async () => {
  const { source, asked } = over({ devices: ok(DEVICES) });
  await vi.advanceTimersByTimeAsync(0);
  source.unpair("d2");
  await vi.advanceTimersByTimeAsync(0);
  expect(asked).toContainEqual({ op: "unpair", id: "d2" });
  expect(asked.at(-1)).toEqual({ op: "devices" });
  const gone = over({ unpair: DOWN });
  await vi.advanceTimersByTimeAsync(0);
  gone.source.unpair("d1");
  await vi.advanceTimersByTimeAsync(0);
  expect(gone.source.get().gateway).toEqual({ state: "not_running" });
});

test("a card showing a problem looks again, so the Gateway starting clears it", async () => {
  let up = false;
  const source = createPhoneGatewaySource(async (request) => (up ? ok(request.op === "devices" ? [] : null) : DOWN));
  source.subscribe(() => undefined);
  await vi.advanceTimersByTimeAsync(0);
  expect(source.get().gateway).toEqual({ state: "not_running" });
  up = true;
  await vi.advanceTimersByTimeAsync(5000);
  expect(source.get().gateway).toEqual({ state: "running" });
});
