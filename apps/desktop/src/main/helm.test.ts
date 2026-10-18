// Which repository Helm answers for, and the requests into its Session, against a real HTTP server on a
// port of its own: what is under test is the aim, the bodies each request sends and the Job record a
// request answers with being handed on. Helm's thread is not here — it is the Sessions host's.

import { once } from "node:events";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import type { HelmContext, RepositorySummary, SessionRecord, SessionTag, SessionUpload, ThoughtTarget } from "@armada/protocol";
import type { HelmAim } from "../shared/api/helm";
import { HelmConnection } from "./helm";

const MANIFEST = "01M2ARMADA0000000000000000";
const OTHER = "01M2SHOP00000000000000000A";
const REPOSITORIES: RepositorySummary[] = [
  { root: "/repos/armada", records_root: "/records/armada", manifest: { id: MANIFEST } as never },
  { root: "/repos/shop", records_root: "/records/shop", manifest: { id: OTHER } as never },
];
const A_SESSION = { id: "01HELMSESSION", helm: true, manifest_id: MANIFEST, state: "live" } as unknown as SessionRecord;

const opened: (() => void)[] = [];
afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** A Fleet answering every request with `answer`, 200. `requests` and `bodies` run parallel. */
async function serving(
  requests: { url: string; method: string }[] = [],
  bodies: string[] = [],
  answer: unknown = A_SESSION,
): Promise<{ port: number; server: Server }> {
  const server = createServer((request, response) => {
    requests.push({ url: request.url ?? "", method: request.method ?? "" });
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      bodies.push(Buffer.concat(chunks).toString("utf8"));
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(answer));
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, server };
}

/** Helm over a Fleet on `port`, pointed at armada, with what it published and the Sessions it handed on. */
function connected(port: number | null) {
  const aims: HelmAim[] = [];
  const folded: SessionRecord[] = [];
  const helm = new HelmConnection({ publish: ({ helm: aim }) => aims.push(aim), port: () => port, folded: (record) => folded.push(record) });
  helm.onRepositoriesChanged(REPOSITORIES);
  return { helm, aims, folded };
}

const SHOT: SessionUpload = { name: "Screenshot.png", media_type: "image/png", data: "iVBORw0KGgo=" };
const ISSUE: SessionTag = { kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" };

describe("asking Helm", () => {
  it("posts to the repository's ask and hands the Session it answers to the Sessions host", async () => {
    const requests: { url: string; method: string }[] = [];
    const fleet = await serving(requests);
    const { helm, folded } = connected(fleet.port);

    expect(await helm.askHelm("Why did job 12 stall?")).toEqual({ ok: true });
    expect(requests).toEqual([{ url: `/helm/ask?manifest_id=${MANIFEST}`, method: "POST" }]);
    expect(folded).toEqual([A_SESSION]);
  });

  // #1075: Bridge sends where the person is with every ask.
  it("sends the context with an ask, and sends none where none was given", async () => {
    const bodies: string[] = [];
    const fleet = await serving([], bodies);
    const { helm } = connected(fleet.port);

    const context: HelmContext = { screen: "manifest", picked: "armada" };
    expect(await helm.askHelm("what is this stuck on?", context)).toEqual({ ok: true });
    expect(await helm.askHelm("and this one?")).toEqual({ ok: true });

    expect(JSON.parse(bodies[0]!)).toEqual({ text: "what is this stuck on?", context });
    expect(JSON.parse(bodies[1]!)).toEqual({ text: "and this one?" });
  });

  it("sends what an ask carried beside its words, and leaves the fields out when it carried none", async () => {
    const bodies: string[] = [];
    const fleet = await serving([], bodies);
    const { helm } = connected(fleet.port);

    expect(await helm.askHelm("do these explain it?", undefined, { attachments: [SHOT], mentions: [ISSUE] })).toEqual({ ok: true });
    expect(await helm.askHelm("and this?", undefined, { attachments: [], mentions: [] })).toEqual({ ok: true });

    expect(JSON.parse(bodies[0]!)).toEqual({ text: "do these explain it?", attachments: [SHOT], mentions: [ISSUE] });
    expect(JSON.parse(bodies[1]!)).toEqual({ text: "and this?" });
  });

  it("names the model and effort the box picked for the message that starts the Session, and nothing for Auto", async () => {
    const bodies: string[] = [];
    const fleet = await serving([], bodies);
    const { helm } = connected(fleet.port);

    expect(await helm.askHelm("first", undefined, { model: "sonnet", effort: "high" })).toEqual({ ok: true });
    expect(await helm.askHelm("second", undefined, { effort: "low" })).toEqual({ ok: true });

    expect(JSON.parse(bodies[0]!)).toEqual({ text: "first", model: "sonnet", effort: "high" });
    expect(JSON.parse(bodies[1]!)).toEqual({ text: "second", effort: "low" });
  });

  it("sends blank words when something is carried, and sends nothing when nothing is", async () => {
    const requests: { url: string; method: string }[] = [];
    const bodies: string[] = [];
    const fleet = await serving(requests, bodies);
    const { helm } = connected(fleet.port);

    expect(await helm.askHelm("  ", undefined, { mentions: [ISSUE] })).toEqual({ ok: true });
    expect(JSON.parse(bodies[0]!)).toEqual({ text: "  ", mentions: [ISSUE] });

    expect(await helm.askHelm("   ")).toEqual({ ok: false, why: "empty_brief" });
    expect(await helm.askHelm("", undefined, { attachments: [], mentions: [] })).toEqual({ ok: false, why: "empty_brief" });
    expect(requests).toHaveLength(1);
  });

  it("says Fleet is not connected without a port, rather than trying to reach one", async () => {
    const { helm, folded } = connected(null);
    expect(await helm.askHelm("hello")).toEqual({ ok: false, why: "not_connected" });
    expect(await helm.startFresh()).toEqual({ ok: false, why: "not_connected" });
    expect(await helm.debugInfo()).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
    expect(folded).toEqual([]);
  });
});

describe("Start fresh and the record", () => {
  it("ends the Helm Session of the repository Helm answers for", async () => {
    const requests: { url: string; method: string }[] = [];
    const fleet = await serving(requests, [], { manifest_id: MANIFEST });
    const { helm } = connected(fleet.port);

    expect(await helm.startFresh()).toEqual({ ok: true });
    expect(requests).toEqual([{ url: `/helm/start_fresh?manifest_id=${MANIFEST}`, method: "POST" }]);
  });

  it("reads the record of that repository's Session, and publishes nothing", async () => {
    const requests: { url: string; method: string }[] = [];
    const record = { manifest_id: MANIFEST, note: "You are Helm." };
    const fleet = await serving(requests, [], record);
    const { helm, aims } = connected(fleet.port);
    const before = aims.length;

    expect(await helm.debugInfo()).toEqual({ ok: true, record });
    expect(requests).toEqual([{ url: `/helm/debug?manifest_id=${MANIFEST}`, method: "GET" }]);
    expect(aims).toHaveLength(before);
  });
});

describe("Helm reading a thought", () => {
  const TARGET: ThoughtTarget = { kind: "session", id: "01SESSION", title: "Sessions list views", manifest_id: MANIFEST, slot: 3 };

  it("posts the words to read_thought and answers what Fleet read, naming no repository", async () => {
    const requests: { url: string; method: string }[] = [];
    const bodies: string[] = [];
    const reading = { kind: "thought", target: TARGET };
    const fleet = await serving(requests, bodies, reading);
    const { helm } = connected(fleet.port);

    expect(await helm.readThought("the sessions page should say waiting")).toEqual({ ok: true, reading });
    expect(requests).toEqual([{ url: "/helm/read_thought", method: "POST" }]);
    expect(JSON.parse(bodies[0]!)).toEqual({ text: "the sessions page should say waiting" });
  });

  it("says a read that could not be made was not, so the dock asks Helm", async () => {
    expect(await connected(null).helm.readThought("anything")).toEqual({ ok: false, outcome: { ok: false, why: "not_connected" } });
  });

  it("records the words that were delivered against the Session the dock shows, and hands on the Session Fleet answers", async () => {
    const requests: { url: string; method: string }[] = [];
    const bodies: string[] = [];
    const fleet = await serving(requests, bodies);
    const { helm, folded } = connected(fleet.port);

    expect(await helm.recordThoughtSent(MANIFEST, "the words", TARGET)).toEqual({ ok: true });
    expect(requests).toEqual([{ url: `/helm/thought_sent?manifest_id=${MANIFEST}`, method: "POST" }]);
    expect(JSON.parse(bodies[0]!)).toEqual({ text: "the words", target: TARGET });
    expect(folded).toEqual([A_SESSION]);
    expect(await connected(null).helm.recordThoughtSent(MANIFEST, "x", TARGET)).toEqual({ ok: false, why: "not_connected" });
  });

  it("records what the thought went with beside its words, and leaves the fields out when it went with none", async () => {
    const bodies: string[] = [];
    const fleet = await serving([], bodies);
    const { helm } = connected(fleet.port);

    expect(await helm.recordThoughtSent(MANIFEST, "the words", TARGET, { attachments: [SHOT], mentions: [ISSUE] })).toEqual({ ok: true });
    expect(await helm.recordThoughtSent(MANIFEST, "the words", TARGET, { attachments: [], mentions: [] })).toEqual({ ok: true });
    expect(JSON.parse(bodies[0]!)).toEqual({ text: "the words", target: TARGET, attachments: [SHOT], mentions: [ISSUE] });
    expect(JSON.parse(bodies[1]!)).toEqual({ text: "the words", target: TARGET });
  });
});

describe("which repository Helm answers for", () => {
  it("publishes the aim when it moves, and not again when it does not", () => {
    const { helm, aims } = connected(null);
    expect(aims).toEqual([{ state: "pointed", manifestId: MANIFEST }]);
    helm.onRepositoriesChanged(REPOSITORIES);
    expect(aims).toHaveLength(1);
    helm.point(OTHER);
    expect(aims.at(-1)).toEqual({ state: "pointed", manifestId: OTHER });
    helm.onRepositoriesChanged([]);
    expect(aims.at(-1)).toEqual({ state: "pointed", manifestId: OTHER });
  });

  it("is pointed at none while no repository has a Manifest", () => {
    const helm = new HelmConnection({ publish: () => {}, port: () => null, folded: () => {} });
    expect(helm.target()).toBeNull();
  });

  it("follows a pick made after a point", () => {
    const { helm } = connected(null);
    helm.point(OTHER);
    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);
  });

  // #948's bug: Discuss on another card was ignored whenever a specific
  // repository was already picked, because the pick was given a fixed
  // priority over a point rather than the two being compared by which was
  // most recent.
  it("lets Discuss point Helm at another repository while one stays picked", () => {
    const { helm } = connected(null);
    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);

    helm.point(OTHER);
    expect(helm.target()).toBe(OTHER);
  });

  it("a pick made after Discuss is the most recent act again", () => {
    const { helm } = connected(null);
    helm.point(OTHER);
    expect(helm.target()).toBe(OTHER);

    helm.onPicked(REPOSITORIES[0]!.root);
    expect(helm.target()).toBe(MANIFEST);
  });

  it("moving to All is not itself an act — the most recent pick or point still stands", () => {
    const { helm } = connected(null);
    helm.onPicked(REPOSITORIES[0]!.root);
    helm.onPicked(null);
    expect(helm.target()).toBe(MANIFEST);
  });

  it("falls back to the first repository with a Manifest where nothing has talked to it yet", () => {
    const { helm } = connected(null);
    helm.onPicked(null);
    expect(helm.target()).toBe(MANIFEST);
  });
});
