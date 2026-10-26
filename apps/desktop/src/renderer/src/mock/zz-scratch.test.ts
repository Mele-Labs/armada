import { test } from "vitest";
import { SCENARIOS } from "./scenario";
test("x", () => {
  const seen = new Map<string, number>();
  for (const s of SCENARIOS) seen.set(String(s?.name), (seen.get(String(s?.name)) ?? 0) + 1);
  console.log("DUPS", JSON.stringify([...seen].filter(([, n]) => n > 1)), "NOSTATE", SCENARIOS.filter((s) => s?.state === undefined).map((s) => s?.name));
});
