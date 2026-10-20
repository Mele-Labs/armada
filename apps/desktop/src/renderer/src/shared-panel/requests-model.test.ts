import { expect, test } from "vitest";

import { bodyShown, environmentsOf, headersOf, headersText, historyOf, resolvedOf, startOf } from "./requests-model";

const STAGING = { name: "staging", base: "https://api.example.test" };
const WEB = { name: "server:web", base: "http://127.0.0.1:5173" };

test("headers are one to a line, Name: value, and a line without a name is named back", () => {
  const headers = [
    { name: "Content-Type", value: "application/json" },
    { name: "X-Trace", value: "a:b" },
  ];
  expect(headersOf(headersText(headers))).toEqual({ headers });
  expect(headersOf("\nAccept: */*\n\n")).toEqual({ headers: [{ name: "Accept", value: "*/*" }] });
  expect(headersOf("Accept */*")).toEqual({ bad: "Accept */*" });
  expect(headersOf(": value")).toEqual({ bad: ": value" });
});

test("the picker offers the saved environments then the Job's servers, and none of a file that did not read", () => {
  expect(environmentsOf({ saved: { what: "read", file: { version: 1, environment: [STAGING] } }, servers: [WEB] })).toEqual([STAGING, WEB]);
  expect(environmentsOf({ saved: { what: "not_read", why: "line 2" }, servers: [WEB] })).toEqual([WEB]);
  expect(environmentsOf(undefined)).toEqual([]);
});

test("{{base}} resolves against the environment picked, and an address without it resolves to nothing", () => {
  expect(resolvedOf("{{base}}/graphql", "staging", [STAGING])).toBe("https://api.example.test/graphql");
  expect(resolvedOf("{{base}}/graphql", "", [STAGING])).toBeUndefined();
  expect(resolvedOf("https://api.example.test/graphql", "staging", [STAGING])).toBeUndefined();
});

test("a row's resolved address reads {{base}} again where it starts with its environment's base", () => {
  const row = { method: "POST", url: "https://api.example.test/graphql", environment: "staging", headers: [], body: "{}" };
  expect(startOf(row, [STAGING])).toEqual({ method: "POST", url: "{{base}}/graphql", environment: "staging", headers: [], body: "{}" });
  expect(startOf(row, [])).toMatchObject({ url: "https://api.example.test/graphql", environment: "staging" });
  expect(startOf({ ...row, url: "https://other.example.test/x" }, [STAGING]).url).toBe("https://other.example.test/x");
});

test("a JSON body is set out two deep, and anything else is drawn as it came", () => {
  expect(bodyShown('{"data":{"me":null}}')).toEqual({ text: '{\n  "data": {\n    "me": null\n  }\n}', json: true });
  expect(bodyShown("<html>")).toEqual({ text: "<html>", json: false });
  expect(bodyShown("{not json")).toEqual({ text: "{not json", json: false });
});

test("a Job's history is its requested rows, newest first", () => {
  const rows = [
    { id: "a", at: "1", kind: "requested", by: "agent", method: "GET", url: "u", headers: [], status: 401, ms: 3, response: "", truncated: false },
    { id: "m", at: "2", kind: "message", from: { kind: "agent" }, text: "t" },
    { id: "b", at: "3", kind: "requested", by: "owner", method: "GET", url: "u", headers: [], status: 200, ms: 3, response: "", truncated: false },
  ];
  expect(historyOf({ rows } as never).map((one) => one.id)).toEqual(["b", "a"]);
  expect(historyOf(undefined)).toEqual([]);
});
