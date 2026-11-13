// The notice over Fleet offers one button, and which one is what is wrong: Start Fleet when no Fleet runs,
// Restart Fleet when Fleet is the stale side of a mismatch, Reopen Armada when this Bridge is.

import { FleetNotice, fleetFailure, statementOf } from "@armada/shell";
import type { BridgeIdentity, Connection, FleetIdentity, KeptBuilds } from "@armada/protocol";
import { createRoot } from "react-dom/client";
import { act } from "react";
import { expect, test, vi } from "vitest";

import "./styles/index.css";

const BRIDGE: BridgeIdentity = {
  auditPath: null,
  fleetProtocol: null,
};
const OLD = "2026-11-12T211515Z-2f88990";
const NEW = "2026-11-13T004852Z-126b4bb";

const FLEET: FleetIdentity = {
  protocolId: "3fa9c1d20b7e4a15",
  pid: 61372,
  port: 40000,
  startedAt: "Thu Nov 12 19:49:00 2026",
};

const DOWN: Connection = {
  state: "not_running",
  absence: {
    why: "no_runtime_file",
    path: "/Users/o/Library/Application Support/Armada/fleet.json",
  },
};

const apart = (build: string): Connection => ({
  state: "protocol_mismatch",
  fleet: {
    ...FLEET,
    build,
  },
  speaks: FLEET.protocolId,
  expected: "91bb07e4c25d8830",
});

/** The notice for `connection`, drawn with the builds this Bridge would read, and the buttons it shows. */
async function drawn(connection: Connection, builds: KeptBuilds) {
  const fleet = fleetFailure(
    connection,
    statementOf(connection, Date.now(), null),
    BRIDGE,
    Date.now(),
  );
  if (fleet === null) throw new Error("a Fleet that is not answering has a notice");
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const start = vi.fn(async () => ({
    ok: true as const,
  }));
  const restart = vi.fn(async () => ({
    ok: true as const,
  }));
  const reopen = vi.fn(async () => ({
    ok: true as const,
  }));
  await act(async () => {
    root.render(
      <FleetNotice
        fleet={fleet}
        fleetPid={"fleet" in connection ? connection.fleet.pid : null}
        fleetBuild={"fleet" in connection ? (connection.fleet.build ?? null) : null}
        bridge={BRIDGE}
        onCopied={() => {}}
        start={start}
        restart={restart}
        reopen={reopen}
        builds={async () => builds}
      />,
    );
  });
  const buttons = () => [
    ...host.querySelectorAll("button"),
  ];
  const named = (name: string) => buttons().find((one) => one.textContent?.trim() === name);
  const done = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return {
    named,
    start,
    restart,
    reopen,
    done,
  };
}

test("no Fleet running offers Start Fleet, and pressing it starts Fleet", async () => {
  const notice = await drawn(DOWN, {
    bridge: null,
    current: null,
  });
  expect(notice.named("Start Fleet")).toBeDefined();
  expect(notice.named("Restart Fleet")).toBeUndefined();
  await act(async () => notice.named("Start Fleet")?.click());
  expect(notice.start).toHaveBeenCalledTimes(1);
  expect(notice.named("Starting Fleet")?.disabled).toBe(true);
  await notice.done();
});

test("a Fleet on an older kept build offers Restart Fleet", async () => {
  const notice = await drawn(apart(OLD), {
    bridge: NEW,
    current: NEW,
  });
  expect(notice.named("Restart Fleet")).toBeDefined();
  expect(notice.named("Reopen Armada")).toBeUndefined();
  await act(async () => notice.named("Restart Fleet")?.click());
  expect(notice.restart).toHaveBeenCalledTimes(1);
  await notice.done();
});

test("a Bridge on an older kept build offers Reopen Armada", async () => {
  const notice = await drawn(apart(NEW), {
    bridge: OLD,
    current: NEW,
  });
  expect(notice.named("Reopen Armada")).toBeDefined();
  expect(notice.named("Restart Fleet")).toBeUndefined();
  await act(async () => notice.named("Reopen Armada")?.click());
  expect(notice.reopen).toHaveBeenCalledTimes(1);
  await notice.done();
});

test("a mismatch with neither side from a kept build offers Restart Fleet, as it always has", async () => {
  const notice = await drawn(apart(NEW), {
    bridge: null,
    current: null,
  });
  expect(notice.named("Restart Fleet")).toBeDefined();
  await notice.done();
});

test("a Start Fleet the command refused shows what it said", async () => {
  const fleet = fleetFailure(DOWN, statementOf(DOWN, Date.now(), null), BRIDGE, Date.now());
  if (fleet === null) throw new Error("down has a notice");
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <FleetNotice
        fleet={fleet}
        fleetPid={null}
        fleetBuild={null}
        bridge={BRIDGE}
        onCopied={() => {}}
        start={async () => ({
          ok: false,
          why: "refused",
          detail:
            "no job file at /Users/o/Library/Application Support/Armada/com.armada.fleet.plist",
        })}
        restart={async () => ({
          ok: true,
        })}
        reopen={async () => ({
          ok: true,
        })}
        builds={async () => ({
          bridge: null,
          current: null,
        })}
      />,
    );
  });
  const press = [
    ...host.querySelectorAll("button"),
  ].find((one) => one.textContent?.trim() === "Start Fleet");
  await act(async () => press?.click());
  expect(host.textContent).toContain("Fleet did not start");
  expect(host.textContent).toContain("com.armada.fleet.plist");
  await act(async () => root.unmount());
  host.remove();
});
