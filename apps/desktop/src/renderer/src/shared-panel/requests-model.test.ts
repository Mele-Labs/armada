import { expect, test } from "vitest";

import { bodyShown, filledOf, headersOf, headersText, headersWith, historyOf, namesIn, repositoryToRead, startOf, unreadFileOf, yoursUnread } from "./requests-model";

const STAGING = { name: "staging", variables: [{ name: "base", secret: false as const, value: "https://api.example.test" }, { name: "token", secret: true as const, set: true }] };

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

test("a plain variable fills its {{name}}, a secret stays a name, and a name the environment lacks is listed", () => {
  expect(filledOf("{{base}}/orders", STAGING)).toEqual({ text: "https://api.example.test/orders", missing: [] });
  expect(filledOf("Bearer {{token}}", STAGING)).toEqual({ text: "Bearer {{token}}", missing: [] });
  expect(filledOf("{{base}}/{{tenant}}", STAGING)).toEqual({ text: "https://api.example.test/{{tenant}}", missing: ["tenant"] });
  expect(filledOf("{{base}}/x", undefined).missing).toEqual(["base"]);
  expect(namesIn("{{base}}/{{ id }}/{{base}}")).toEqual(["base", "id"]);
});

test("a collection's headers go under a request's own of the same name", () => {
  const shared = [{ name: "Authorization", value: "Bearer {{token}}" }, { name: "Accept", value: "*/*" }];
  expect(headersWith(shared, [{ name: "accept", value: "application/json" }])).toEqual([{ name: "Authorization", value: "Bearer {{token}}" }, { name: "accept", value: "application/json" }]);
});

test("a row's filled address reads {{base}} again where it starts with its environment's base", () => {
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

test("a file that would not read is named by where it is kept, and only the person's own stops a save", () => {
  const read = { repository: "/srv/store", place: "this_mac" as const, collections: [], environments: [] };
  const theirs = { place: "repository" as const, why: "line 4: expected a table" };
  const yours = { place: "yours" as const, why: "version 3 is newer than this Fleet reads" };
  expect(unreadFileOf(theirs)).toBe("The repository's .armada/requests.toml");
  expect(unreadFileOf(yours)).toBe("Your own requests file");
  expect(yoursUnread({ ...read, not_read: [theirs] })).toBeUndefined();
  expect(yoursUnread({ ...read, not_read: [theirs, yours] })).toEqual(yours);
});

test("with no Job open, Requests reads the Cockpit's repository, else the one picked last, else the first Fleet lists", () => {
  const listed = ["/srv/store", "/srv/billing", "/srv/docs"];
  expect(repositoryToRead(listed, "/srv/billing", ["/srv/docs"])).toBe("/srv/billing");
  expect(repositoryToRead(listed, null, ["/srv/gone", "/srv/docs"])).toBe("/srv/docs");
  expect(repositoryToRead(listed, "/srv/gone", [])).toBe("/srv/store");
  expect(repositoryToRead([], null, ["/srv/docs"])).toBeUndefined();
});
