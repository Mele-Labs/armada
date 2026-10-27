// The Fleet-backed shared home, against a fake Fleet and a fake account: whose home is read, how a row on the wire
// becomes Kit's, Sync turning while it runs, the three acts naming the row Fleet takes, a refusal told as a press that
// failed, reading again after What syncs moves, and What syncs switched from Kit. What the marks look like is the composition's (`KitBrowser`).

import { describe, expect, it } from "vitest";

import type { Profile } from "@armada/components";
import type { HomeItem, HomeKind, HomeKinds, HomeRow, HomeRowAt, HomeSynced, Outcome, SharedHomeRead } from "@armada/protocol";

import type { AccountsActed } from "../../shared/api/accounts";
import type { AccountHeld, AccountSource } from "./fleet-account";
import { createFleetSharedHome, type FleetHome } from "./fleet-shared-home";

const settled = () => new Promise((done) => setTimeout(done, 0));

const profile = (login: string, chosen: boolean, orgs?: string[]): Profile => ({ account_id: chosen ? 2 : 1, login, chosen, active: false, ...(orgs === undefined ? {} : { orgs }) });

/** An account source holding `profiles`, which the test moves. */
function account(profiles: Profile[] | undefined) {
  let now = profiles;
  let refreshed = 0;
  const listeners = new Set<() => void>();
  const source: AccountSource = {
    refresh: () => void refreshed++,
    get: () => (now === undefined ? undefined : ({ account: { profiles: now } } as unknown as AccountHeld)),
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    signOut: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }),
  };
  return { source, refreshed: () => refreshed, become: (to: Profile[] | undefined) => ((now = to), listeners.forEach((on) => on())) };
}

const SERVER: HomeRow = { kind: "mcp_servers", name: "linear", state: "home", source: "mcp.linear.app", key_here: false };
const NOTE: HomeRow = { kind: "memory", name: "Walks, not pictures", state: "here", repository: "armada" };
const DUSK: HomeRow = { kind: "mods", name: "dusk", state: "differs", here: "--accent: #E8A25C;", there: "--accent: #D9896B;" };
const HELD: SharedHomeRead = { home: { account: "nickmele", repository: "armada-home" }, rows: [SERVER, NOTE, DUSK], unreadable: [] };
const refusal = (message: string): Outcome => ({ ok: false, why: "refused", error: { code: "fleet.home_login_not_held", message, run_id: "r1", fields: {}, chain: [] } });

const KINDS: readonly HomeKind[] = ["skills", "plugins", "global_instructions", "sub_agents", "commands", "mcp_servers", "allowlist", "workflows", "memory", "mods"];

/** A Fleet holding `read` and What syncs (`on` kinds, `items`), a record of what Bridge asked of it, and a Sync that answers when the test says. */
function fake(read: SharedHomeRead, over: Partial<FleetHome> = {}, on: HomeKind[] = ["skills"], items: HomeItem[] = []) {
  const asked: string[] = [];
  const kinds = (login: string): AccountsActed<HomeKinds> => ({ ok: true, value: { login, kinds: KINDS.map((kind) => ({ kind, on: on.includes(kind) })), items: [...items] } });
  let synced: ((answer: AccountsActed<HomeSynced>) => void) | undefined;
  const settle = (at: HomeRowAt): AccountsActed<SharedHomeRead> => ({ ok: true, value: { ...read, rows: read.rows.map((one) => (one.name === at.name ? { kind: one.kind, name: one.name, state: "same" } : one)) } });
  const fleet: FleetHome = {
    getSharedHome: async (login) => (asked.push(`read ${login}`), { ok: true, value: read }),
    syncSharedHome: (login) => (asked.push(`sync ${login}`), new Promise((done) => (synced = done))),
    addToSharedHome: async (at) => (asked.push(`add ${JSON.stringify(at)}`), settle(at)),
    bringFromSharedHome: async (at) => (asked.push(`bring ${JSON.stringify(at)}`), settle(at)),
    chooseSharedHomeSide: async (at, side) => (asked.push(`choose ${JSON.stringify(at)} ${side}`), settle(at)),
    setHomeRepository: (login, owner, name) => (asked.push(`repository ${login} ${owner}/${name}`), Promise.resolve({ ok: true, value: { ...read, planned: { account: owner, repository: name, exists: false } } })),
    getHomeKinds: async (login) => kinds(login),
    setHomeKind: async (login, kind, to) => {
      asked.push(`kind ${login} ${kind} ${to}`);
      on = to ? [...on, kind] : on.filter((one) => one !== kind);
      return kinds(login);
    },
    setHomeItem: async (row, to) => {
      asked.push(`item ${JSON.stringify(row)} ${to}`);
      items = to ? [...items, { kind: row.kind, name: row.name }] : items.filter((one) => one.name !== row.name);
      return kinds(row.login);
    },
    ...over,
  };
  return { fleet, asked, answerSync: (answer: AccountsActed<HomeSynced>) => synced?.(answer) };
}

describe("the shared home over Fleet", () => {
  it("draws nothing until a profile is signed in, then the chosen profile's home with Kit's marks", async () => {
    const who = account(undefined);
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, who.source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect(home.get()).toBeUndefined();
    expect(fleet.asked).toEqual([]);

    who.become([profile("mele-work", false), profile("nickmele", true)]);
    await settled();
    expect(fleet.asked).toEqual(["read nickmele"]);
    const drawn = home.get();
    expect([drawn?.account, drawn?.repository, drawn?.syncing]).toEqual(["nickmele", "armada-home", false]);
    // The wire's `key_here: false` is a server whose key is not here: Kit's row needs its key.
    expect(drawn?.rows[0]).toEqual({ kind: "mcp_servers", name: "linear", state: "home", source: "mcp.linear.app", keyHere: true });
    expect(drawn?.rows[1]).toEqual({ kind: "memory", name: "Walks, not pictures", state: "here" });
    expect(drawn?.rows[2]).toEqual({ kind: "mods", name: "dusk", state: "differs", here: "--accent: #E8A25C;", there: "--accent: #D9896B;" });

    who.become(undefined);
    expect(home.get()).toBeUndefined();
  });

  it("names a home this machine holds no copy of by the profile's login, and marks no key on a server that has its own", async () => {
    const fleet = fake({ rows: [{ ...SERVER, key_here: true }], unreadable: [] });
    const home = createFleetSharedHome(fleet.fleet, account([profile("mele-work", false)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect([home.get()?.account, home.get()?.repository]).toEqual(["mele-work", "armada-home"]);
    expect(home.get()?.rows[0]?.keyHere).toBeUndefined();
  });

  it("lists a mod under the kind Fleet read from its mod.toml, and a row with none as before", async () => {
    const deck: HomeRow = { kind: "mods", name: "deck", state: "home", mod_kind: "layout" };
    const links: HomeRow = { kind: "mods", name: "links", state: "home", mod_kind: "words" };
    const fleet = fake({ rows: [deck, links, DUSK], unreadable: [] });
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect(home.get()?.rows.map((row) => row.modKind)).toEqual(["layout", "words", undefined]);
  });

  it("turns while Sync runs and draws what it answers", async () => {
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    home.get()?.sync();
    expect(home.get()?.syncing).toBe(true);
    home.get()?.sync();
    expect(fleet.asked.filter((one) => one.startsWith("sync"))).toEqual(["sync nickmele"]);
    fleet.answerSync({ ok: true, value: { ...HELD, rows: [], left: [] } });
    await settled();
    expect(home.get()?.syncing).toBe(false);
    expect(home.get()?.rows).toEqual([]);
  });

  it("sends each act the row as Fleet names it, a memory note with its repository, and draws the answer", async () => {
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    const [server, note, dusk] = home.get()?.rows ?? [];
    home.get()?.add(note!);
    await settled();
    home.get()?.bring(server!);
    await settled();
    home.get()?.choose(dusk!, "there");
    await settled();
    expect(fleet.asked.slice(1)).toEqual([
      `add ${JSON.stringify({ login: "nickmele", kind: "memory", name: "Walks, not pictures", repository: "armada" })}`,
      `bring ${JSON.stringify({ login: "nickmele", kind: "mcp_servers", name: "linear" })}`,
      `choose ${JSON.stringify({ login: "nickmele", kind: "mods", name: "dusk" })} there`,
    ]);
    expect(home.get()?.rows.find((one) => one.name === "dusk")?.state).toBe("same");
  });

  it("names a row drawn before a read again as it was drawn, as a compare sheet left open holds it", async () => {
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    const note = home.get()?.rows[1];
    home.readAgain("nickmele");
    await settled();
    home.get()?.choose(note!, "here");
    await settled();
    expect(fleet.asked.at(-1)).toBe(`choose ${JSON.stringify({ login: "nickmele", kind: "memory", name: "Walks, not pictures", repository: "armada" })} here`);
  });

  it("tells a refused press in Fleet's words, keeps the rows, and stops turning", async () => {
    const told: string[] = [];
    const said = "gh does not hold nickmele here: run gh auth login --hostname github.com";
    const fleet = fake(HELD, { addToSharedHome: async () => ({ ok: false, outcome: refusal(said) }) });
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, (sentence) => told.push(sentence));
    home.subscribe(() => undefined);
    await settled();
    const before = home.get()?.rows;
    home.get()?.add(before![1]!);
    await settled();
    home.get()?.sync();
    fleet.answerSync({ ok: false, outcome: refusal(said) });
    await settled();
    expect(told).toEqual([said, said]);
    expect(home.get()?.rows).toEqual(before);
    expect(home.get()?.syncing).toBe(false);
  });

  it("reads the drawn home again when What syncs moves for its login, and not for another", async () => {
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, account([profile("mele-work", false), profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    home.readAgain("mele-work");
    home.readAgain("nickmele");
    await settled();
    expect(fleet.asked).toEqual(["read nickmele", "read nickmele"]);
  });

  it("draws the place Fleet plans for a home with no copy here, not yet made, with the profile's organisations as owners", async () => {
    const planned: SharedHomeRead = { planned: { account: "mele-labs", repository: "team-home", exists: false }, rows: [], unreadable: [] };
    const fleet = fake(planned);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true, ["mele-labs", "other"])]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    const drawn = home.get();
    expect([drawn?.account, drawn?.repository, drawn?.made, drawn?.login]).toEqual(["mele-labs", "team-home", false, "nickmele"]);
    expect(drawn?.owners).toEqual(["mele-labs", "other"]);
  });

  it("draws a home made on another Mac as made, with no picker to change it", async () => {
    const fleet = fake({ planned: { account: "mele-labs", repository: "team-home", exists: true }, rows: [], unreadable: [] });
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true, ["mele-labs"])]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    const drawn = home.get();
    expect([drawn?.account, drawn?.repository, drawn?.made, drawn?.owners]).toEqual(["mele-labs", "team-home", undefined, undefined]);
  });

  it("falls back to the login and the default name where Fleet names no place", async () => {
    const fleet = fake({ rows: [], unreadable: [] });
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true, [])]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect([home.get()?.account, home.get()?.repository, home.get()?.made]).toEqual(["nickmele", "armada-home", false]);
  });

  it("sends the chosen owner and name for the drawn login, draws it at once, and takes Fleet's answer", async () => {
    let answer: ((to: AccountsActed<SharedHomeRead>) => void) | undefined;
    const fleet = fake({ planned: { account: "nickmele", repository: "armada-home", exists: false }, rows: [], unreadable: [] }, {
      setHomeRepository: (login, owner, name) => (fleet.asked.push(`repository ${login} ${owner}/${name}`), new Promise((done) => (answer = done))),
    });
    const who = account([profile("nickmele", true, ["mele-labs"])]);
    const home = createFleetSharedHome(fleet.fleet, who.source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    home.get()?.setHome?.("mele-labs", "team-home");
    expect(fleet.asked.at(-1)).toBe("repository nickmele mele-labs/team-home");
    expect([home.get()?.account, home.get()?.repository]).toEqual(["mele-labs", "team-home"]);
    answer?.({ ok: true, value: { planned: { account: "mele-labs", repository: "team-home", exists: false }, rows: [], unreadable: [] } });
    await settled();
    expect([home.get()?.account, home.get()?.repository, home.get()?.made]).toEqual(["mele-labs", "team-home", false]);
    // The Account page names the same place, so it reads again.
    expect(who.refreshed()).toBe(1);
  });

  it("tells a refused choice in Fleet's words and draws the place it stood at", async () => {
    const told: string[] = [];
    const said = "nickmele may not make repositories in mele-labs";
    const fleet = fake({ planned: { account: "nickmele", repository: "armada-home", exists: false }, rows: [], unreadable: [] }, {
      setHomeRepository: async () => ({ ok: false, outcome: refusal(said) }),
    });
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true, ["mele-labs"])]).source, (sentence) => told.push(sentence));
    home.subscribe(() => undefined);
    await settled();
    home.get()?.setHome?.("mele-labs", "team-home");
    expect(home.get()?.account).toBe("mele-labs");
    await settled();
    expect(told).toEqual([said]);
    expect([home.get()?.account, home.get()?.repository]).toEqual(["nickmele", "armada-home"]);
  });

  it("offers no choice once a copy is here", async () => {
    const fleet = fake(HELD);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true, ["mele-labs"])]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect([home.get()?.made, home.get()?.owners]).toEqual([undefined, undefined]);
  });

  it("draws What syncs beside the rows, and sends a kind or an item switched on in Kit to the home at once", async () => {
    const fleet = fake(HELD, {}, ["skills"], [{ kind: "memory", name: "Walks, not pictures", repository: "armada" }]);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, () => undefined);
    home.subscribe(() => undefined);
    await settled();
    expect(home.get()?.syncs).toEqual({ kinds: ["skills"], items: [{ kind: "memory", name: "Walks, not pictures" }] });

    home.get()?.setKind?.("plugins", true);
    await settled();
    expect(home.get()?.syncs?.kinds).toEqual(["skills", "plugins"]);
    expect(home.get()?.syncing).toBe(true);
    fleet.answerSync({ ok: true, value: { ...HELD, left: [] } });
    await settled();
    home.get()?.setItem?.({ kind: "sub_agents", name: "reviewer" }, true);
    await settled();
    expect(home.get()?.syncs?.items).toEqual([
      { kind: "memory", name: "Walks, not pictures" },
      { kind: "sub_agents", name: "reviewer" },
    ]);
    fleet.answerSync({ ok: true, value: { ...HELD, left: [] } });
    await settled();
    // Off sends nothing: the rows are read again.
    home.get()?.setItem?.({ kind: "sub_agents", name: "reviewer" }, false);
    await settled();
    expect(fleet.asked).toEqual([
      "read nickmele",
      "kind nickmele plugins true",
      "sync nickmele",
      `item ${JSON.stringify({ login: "nickmele", kind: "sub_agents", name: "reviewer" })} true`,
      "sync nickmele",
      `item ${JSON.stringify({ login: "nickmele", kind: "sub_agents", name: "reviewer" })} false`,
      "read nickmele",
    ]);
  });

  it("tells that nothing syncs yet when Sync runs with no kind or item switched on", async () => {
    const told: string[] = [];
    const fleet = fake(HELD, {}, []);
    const home = createFleetSharedHome(fleet.fleet, account([profile("nickmele", true)]).source, (sentence) => told.push(sentence));
    home.subscribe(() => undefined);
    await settled();
    home.get()?.sync();
    fleet.answerSync({ ok: true, value: { ...HELD, rows: [], left: [] } });
    await settled();
    expect(told).toEqual(["Nothing syncs yet: no kind or item is switched on"]);
  });
});
