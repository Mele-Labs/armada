import { expect, test } from "vitest";

import { environmentsFileOf, sendOf } from "./request-channels";

test("a send crosses with its own fields only, and anything misshapen is refused as unsent", () => {
  const send = { session_id: "s1", method: "POST", url: "{{base}}/graphql", environment: "staging", headers: [{ name: "Content-Type", value: "application/json" }], body: "{}" };
  expect(sendOf({ ...send, extra: 1 })).toEqual(send);
  expect(sendOf({ method: "GET", url: "https://api.example.test", headers: [] })).toEqual({ method: "GET", url: "https://api.example.test", headers: [] });
  expect(sendOf({ ...send, headers: [{ name: "A" }] })).toBeNull();
  expect(sendOf({ ...send, method: "" })).toBeNull();
  expect(sendOf({ ...send, session_id: 7 })).toBeNull();
  expect(sendOf(null)).toBeNull();
});

test("the environments file crosses whole, and a row without a name and a base is refused", () => {
  const file = { version: 1, environment: [{ name: "staging", base: "https://api.example.test" }] };
  expect(environmentsFileOf(file)).toEqual(file);
  expect(environmentsFileOf({ version: 1, environment: [{ name: "staging" }] })).toBeNull();
  expect(environmentsFileOf({ environment: [] })).toBeNull();
});
