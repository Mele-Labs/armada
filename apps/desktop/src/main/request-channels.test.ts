import { expect, test } from "vitest";

import { collectionOf, environmentOf, sendOf, whereOf } from "./request-channels";
import { requestsQuery } from "./requests";

test("a send crosses with its own fields only, and anything misshapen is refused as unsent", () => {
  const send = { session_id: "s1", repository: "/srv/store", method: "POST", url: "{{base}}/graphql", collection: "store-api", environment: "staging", headers: [{ name: "Content-Type", value: "application/json" }], body: "{}" };
  expect(sendOf({ ...send, extra: 1 })).toEqual(send);
  expect(sendOf({ method: "GET", url: "https://api.example.test", headers: [] })).toEqual({ method: "GET", url: "https://api.example.test", headers: [] });
  expect(sendOf({ ...send, headers: [{ name: "A" }] })).toBeNull();
  expect(sendOf({ ...send, method: "" })).toBeNull();
  expect(sendOf({ ...send, session_id: 7 })).toBeNull();
  expect(sendOf(null)).toBeNull();
});

test("a saved request is sent by its names, and never with its own method or address beside them", () => {
  const saved = { saved: { collection: "store-api", request: "create-order" }, environment: "test", headers: [] };
  expect(sendOf(saved)).toEqual(saved);
  expect(sendOf({ ...saved, method: "GET", url: "https://api.example.test" })).toBeNull();
  expect(sendOf({ ...saved, saved: { collection: "store-api" } })).toBeNull();
  expect(sendOf({ headers: [] })).toBeNull();
});

test("a collection crosses whole with its requests, and one missing a field is refused", () => {
  const collection = {
    id: "store-api",
    name: "Store API",
    headers: [{ name: "Authorization", value: "Bearer {{token}}" }],
    requests: [{ id: "create-order", name: "Create order", method: "POST", url: "{{base}}/orders", headers: [], body: "{}" }, { id: "health", name: "Health", method: "GET", url: "{{base}}/health", headers: [] }],
  };
  expect(collectionOf({ ...collection, place: "yours", hides: false })).toEqual(collection);
  expect(collectionOf({ ...collection, requests: [{ id: "x", name: "X", method: "GET", headers: [] }] })).toBeNull();
  expect(collectionOf({ ...collection, headers: undefined })).toBeNull();
  expect(collectionOf("store-api")).toBeNull();
});

test("an environment crosses with a secret's value toward Fleet, or without one to keep the value kept", () => {
  const environment = {
    name: "test",
    variables: [
      { name: "base", secret: false, value: "https://api.test.example.test" },
      { name: "token", secret: true, value: "typed-once" },
      { name: "key", secret: true },
    ],
  };
  expect(environmentOf(environment)).toEqual(environment);
  expect(environmentOf({ name: "test", variables: [{ name: "base", secret: false }] })).toBeNull();
  expect(environmentOf({ name: "test", variables: [{ name: "token", secret: "yes", value: "x" }] })).toBeNull();
  expect(environmentOf({ variables: [] })).toBeNull();
});

test("a read or save names a Job, a repository, both or neither, and nothing else", () => {
  expect(whereOf("s1", "/srv/store")).toEqual({ session_id: "s1", repository: "/srv/store" });
  expect(whereOf(undefined, undefined)).toEqual({});
  expect(whereOf("", undefined)).toBeNull();
  expect(whereOf(undefined, 7)).toBeNull();
  expect(requestsQuery({ repository: "/srv/my store", session_id: "s1" })).toBe("?repository=%2Fsrv%2Fmy+store&session_id=s1");
  expect(requestsQuery({})).toBe("");
});
