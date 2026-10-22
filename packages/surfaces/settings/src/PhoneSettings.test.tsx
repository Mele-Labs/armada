// Settings → Phone over the real source, with Fleet and the Gateway answering by operation. The claim is
// what the card draws for each answer to `/phone`, and that the switch posts to Fleet.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, unmount } from "@armada/screens/src/mounted";

import type { PhoneAnswer, PhoneRequest } from "./api";
import { createPhoneGatewaySource } from "./phone-gateway-source";
import { PhoneSourceProvider } from "./phone-source";
import { PhoneSettings } from "./PhoneSettings";

afterEach(() => unmount());

const ok = (body: unknown): PhoneAnswer => ({ ok: true, body });

/** The card over a source whose `/phone` answers `access` first and `turned` after a press. */
function card(access: unknown, turned: unknown = access) {
  const asked: PhoneRequest[] = [];
  const source = createPhoneGatewaySource(async (request) => {
    asked.push(request);
    if (request.op === "access") return ok(access);
    if (request.op === "turn") return ok(turned);
    if (request.op === "devices") return ok([{ id: "d1", name: "iPhone", created_at: 100, last_seen_at: 200 }]);
    return ok(null);
  });
  mount(
    <PhoneSourceProvider value={source}>
      <PhoneSettings />
    </PhoneSourceProvider>,
  );
  return asked;
}

const toggle = () => page.getByRole("switch", { name: "Phone access" });

test("access off draws the switch and no pairing, and pressing it posts on", async () => {
  const asked = card({ on: false, port: 8443, listening: false }, { on: true, port: 8443, listening: true });
  await expect.element(toggle()).not.toBeChecked();
  expect(page.getByRole("button", { name: "Pair a phone" }).elements()).toHaveLength(0);
  expect(page.getByText("iPhone").elements()).toHaveLength(0);

  await userEvent.click(toggle());
  await expect.element(toggle()).toBeChecked();
  expect(asked).toContainEqual({ op: "turn", on: true });
  await expect.element(page.getByRole("button", { name: "Pair a phone" })).toBeVisible();
  await expect.element(page.getByText("iPhone")).toBeVisible();
});

test("access on with nothing listening shows Fleet's sentence in place of pairing", async () => {
  const why = "Port 8443 is in use by another program.";
  card({ on: true, port: 8443, listening: false, why });
  await expect.element(toggle()).toBeChecked();
  await expect.element(page.getByText(why)).toBeVisible();
  expect(page.getByRole("button", { name: "Pair a phone" }).elements()).toHaveLength(0);
});
