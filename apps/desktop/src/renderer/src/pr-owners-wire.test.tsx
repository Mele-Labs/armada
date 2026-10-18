// The owner chip drawn from the wire: the `owner` on the merge line's open pull requests, and on the
// panel's page. A pull request Fleet names no owner for draws no chip; the mock's registry, where a
// window holds one, still answers for what the wire does not.

import { act } from "react";
import type { ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { PrOwnerRegistryProvider, RegisteredOwnerOf } from "@armada/components";
import type { PrOwnerRegistry } from "@armada/components";
import type { HubPullRequest, MergeLines, PullRequestOwner } from "@armada/protocol";

import { ownersOnLines, PrOwnersOnTheWire } from "./pr-owners-wire";
import type { OwnedPanel } from "./pr-owners-wire";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const A_SESSION: PullRequestOwner = { kind: "session", id: "s-a1b2", machine: "mac-a", this_machine: false };
const HERE: PullRequestOwner = { kind: "session", id: "s-here", machine: "mac-b", this_machine: true };

const pull = (number: number, owner?: PullRequestOwner): HubPullRequest => ({
  number,
  title: `Change ${number}`,
  branch: `armada/${number}`,
  url: `https://forge.invalid/mele-labs/armada/pull/${number}`,
  ...(owner === undefined ? {} : { owner }),
});

const linesOf = (...pulls: HubPullRequest[]): MergeLines => ({
  lines: [{ root: "/srv/armada", line: [], off: [], landed: [], sent_back: [], hub: { pull_requests: pulls } }],
});

/** The chips for pull requests 30, 31 and 32 of the repository, under whatever `around` wraps them in. */
function window(lines: MergeLines | null, panel?: OwnedPanel, around?: (inner: ReactElement) => ReactElement): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const chips = (
    <PrOwnersOnTheWire lines={lines} {...(panel === undefined ? {} : { panel })}>
      {[30, 31, 32].map((number) => (
        <section key={number} aria-label={`#${number}`}>
          <RegisteredOwnerOf repository="mele-labs/armada" number={number} />
        </section>
      ))}
    </PrOwnersOnTheWire>
  );
  act(() => root.render(around === undefined ? chips : around(chips)));
  return host;
}

const chipOn = (host: HTMLElement, number: number) => host.querySelector(`section[aria-label="#${number}"] .armada-registered-owner`);

test("a pull request the wire names an owner for draws the chip, and one it does not draws none", () => {
  const host = window(linesOf(pull(30, A_SESSION), pull(31)));
  expect(chipOn(host, 30)?.getAttribute("aria-label")).toBe("Job s-a1b2, mac-a");
  expect(chipOn(host, 31)).toBeNull();
  expect(chipOn(host, 32)).toBeNull();
});

test("a row written by this machine reads as this machine, and another's names its machine", () => {
  const host = window(linesOf(pull(30, HERE), pull(31, A_SESSION)));
  expect(chipOn(host, 30)?.getAttribute("aria-label")).toBe("Job s-here, This machine");
  expect(chipOn(host, 30)?.querySelector("[data-here]")).not.toBeNull();
  expect(chipOn(host, 31)?.querySelector("[data-here]")).toBeNull();
});

test("a Fleet that served no owners, or no lines, draws no chip", () => {
  expect(chipOn(window(linesOf(pull(30), pull(31))), 30)).toBeNull();
  expect(chipOn(window(null), 30)).toBeNull();
});

test("the panel's page names the owner of its own pull request over the lines", () => {
  const host = window(linesOf(pull(30, A_SESSION)), { repository: "mele-labs/armada", number: 30, owner: HERE });
  expect(chipOn(host, 30)?.getAttribute("aria-label")).toBe("Job s-here, This machine");
});

test("a page with no owner leaves the one the lines carry", () => {
  const host = window(linesOf(pull(30, A_SESSION)), { repository: "mele-labs/armada", number: 30, owner: undefined });
  expect(chipOn(host, 30)?.getAttribute("aria-label")).toBe("Job s-a1b2, mac-a");
});

test("owners are read off every line's pull requests by repository and number", () => {
  const owners = ownersOnLines(linesOf(pull(30, A_SESSION), pull(31)));
  expect([...owners.keys()]).toEqual(["mele-labs/armada#30"]);
});

test("a registry the window already holds answers for what the wire does not", () => {
  const mock: PrOwnerRegistry = {
    ownerOf: (_, number) => (number === 32 ? { kind: "session", id: "mock-session", machine: { name: "mock-mac", here: false } } : undefined),
    registering: true,
    setRegistering: () => undefined,
  };
  const host = window(linesOf(pull(30, A_SESSION)), undefined, (inner) => <PrOwnerRegistryProvider value={mock}>{inner}</PrOwnerRegistryProvider>);
  expect(chipOn(host, 30)?.getAttribute("aria-label")).toBe("Job s-a1b2, mac-a");
  expect(chipOn(host, 32)?.getAttribute("aria-label")).toBe("Job mock-session, mock-mac");
  expect(chipOn(host, 31)).toBeNull();
});
