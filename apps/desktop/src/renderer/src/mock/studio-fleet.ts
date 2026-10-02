// A Fleet keeping this repository's Studios: started, a node moved or removed, a relation decided,
// each written and published whole the way `studio.changed` carries it — #1287. `helmProposes` is
// the one write a person does not make, standing in for Helm adding nodes and proposing a relation.
//
// **`keeping()` is every scenario's, and `studying()` is one scenario** — #1341. The store answers
// the Studio reads and writes wherever `fake.ts` puts it, so a scenario that keeps none answers an
// empty list and the surface draws its empty state; the `studios` scenario adds the Helm write.
//
// **Promotion is the same shape** — #1291: each rung adds the node it makes and the `produced`
// edges the Studio draws, and answers with the Studio whole. **No proposer runs here**, so a
// dispatch mints one Job node: what the mock proves is the surface, and which workflow a model
// picks is `fleet`'s.

import type {
  EpicTake,
  Outcome,
  SketchDrawing,
  SketchToKeep,
  Studio,
  StudioCapture,
  StudioEdge,
  StudioNode,
  StudioNodeByHand,
  StudioNodeContent,
  StudioPromotion,
} from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";
import { foldStudio } from "@armada/screens/src/studio-reads";

import type { BridgeApi } from "../../../shared/api";
import { sheet } from "./manifest-fleet";
import { onBoard, unanswered } from "./moment";
import type { FleetHandle, Scenario } from "./moment";

const OK = { ok: true } as const;
const MANIFEST_ID = repository().manifest!.id;

/** The files a test says are on disk, by path. **The mock has no disk**, so every other one has none. */
const ON_DISK = new WeakMap<File, string>();

/** Say `file` is on disk at `path`, the way Finder's copy of one is. */
export function onDisk(file: File, path: string): File {
  ON_DISK.set(file, path);
  return file;
}

/** `pathOfFile`, as main answers it: the path of a file on disk, and `""` for a screenshot. */
export const pathOnDisk = (file: File): string => ON_DISK.get(file) ?? "";

/** Fleet's bound on a kept frame, `MOST_A_FRAME_MAY_WEIGH` in `crates/fleet/src/studios.rs`. */
export const MOST_A_FRAME_MAY_WEIGH = 4 * 1024 * 1024;

/** A refusal, in Fleet's own code and words. */
const refusedAs = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/** What Fleet answers a picture over that bound with, in Fleet's own words. */
const tooLarge = (weighs: number): Outcome =>
  refusedAs(
    "fleet.studio_frame_too_large",
    `a frame weighs at most ${MOST_A_FRAME_MAY_WEIGH} bytes and this one weighs ${weighs}`,
  );

/**
 * Every Studio call a mock Fleet answers. **Named one by one**, so a Studio capability added to
 * `BridgeApi` fails typecheck here rather than going unanswered at runtime — `fake.ts`'s own rule.
 */
export type StudioRoutes = Pick<
  BridgeApi,
  | "watchStudios"
  | "watchStudio"
  | "createStudio"
  | "pathOfFile"
  | "addStudioPicture"
  | "addStudioSketch"
  | "saveStudioSketch"
  | "renameStudio"
  | "addStudioNode"
  | "captureStudioNote"
  | "readStudioFrame"
  | "moveStudioNode"
  | "removeStudioNodes"
  | "decideStudioEdge"
  | "promoteOnStudio"
  | "openStudioNode"
>;

/** The Studios a mock Fleet keeps, and what Helm writes into one. */
export type StudioKeeping = {
  /** What this Fleet answers the Studio calls with, over the window it publishes to. */
  routes: (fleet: FleetHandle) => StudioRoutes;
  /** Every Studio as Fleet holds it now. */
  studios: () => readonly Studio[];
  /** Helm adds a Note and a Finding to a Studio, and proposes that the Finding answers the Note. */
  helmProposes: (studioId: string) => void;
  /** Two Notes a person captured, which #1291's rungs are worked from. */
  twoNotes: (studioId: string) => void;
  /** One Contradiction, Reported, which a person ends one of four ways. */
  aContradiction: (studioId: string) => void;
  /** Every address handed to the system browser, in the order it was — #1406. */
  browsed: () => readonly string[];
};

/** One scenario's own Fleet, keeping Studios. */
export type StudioFleet = StudioKeeping & { scenario: Scenario };

let minted = 0;
const mint = (prefix: string) => `${prefix}${String(++minted).padStart(4, "0")}`;
/** Each write a second later than the last, so last touched first is a real order. */
let clock = Date.parse("2026-09-17T09:00:00Z");
const tick = () => new Date((clock += 1000)).toISOString();

const at = "2026-09-16T15:30:00Z";

/**
 * The picture a Note kept, as a mock Fleet answers it — #1352.
 *
 * **Drawn here rather than kept as a file.** A mock Fleet has no disk and no
 * window to photograph, and what a scenario has to answer is bytes an `img` can
 * draw; a PNG committed beside this would be a binary nobody can read a diff of.
 * A window's bands, not its words: the colours and the boxes are a photograph's
 * and no token of the design system applies to one.
 */
async function aFrame(): Promise<Uint8Array> {
  const canvas = new OffscreenCanvas(1440, 900);
  const ink = canvas.getContext("2d")!;
  ink.fillStyle = "darkslategray";
  ink.fillRect(0, 0, 1440, 900);
  ink.fillStyle = "slategray";
  ink.fillRect(0, 0, 1440, 72);
  ink.fillStyle = "gainsboro";
  ink.fillRect(64, 160, 420, 560);
  ink.fillRect(548, 160, 828, 260);
  const png = await canvas.convertToBlob({ type: "image/png" });
  return new Uint8Array(await png.arrayBuffer());
}

/** What a Note's `capture` carries where the mock kept a frame for it. */
function pointedAt(nodeId: string): StudioCapture {
  return {
    selector: "button.armada-chip",
    element: { tag: "button", text: "Queued 3" },
    location: "/",
    bounds: { x: 312, y: 148, width: 96, height: 28 },
    window: { width: 1440, height: 900 },
    markup: '<button class="armada-chip">Queued 3</button>',
    frame: { filename: `${nodeId}.png`, byte_size: 41_000, width: 1440, height: 900 },
  };
}

/** A Studio already kept, so the list and the whiteboard have something to draw on the mock page. */
function legend(): Studio {
  return {
    id: "01STUDIOLEGEND0000000000000",
    manifest_id: MANIFEST_ID,
    name: "The Board's legend",
    created_at: at,
    touched_at: at,
    nodes: [
      // One Note with the picture it kept and one without: both are Notes, and only one draws a plate.
      { id: "legend-note", kind: "note", said: "The legend under the step bar is unreadable", capture: pointedAt("legend-note"), position: { x: 0, y: 0 }, created_at: at },
      { id: "legend-width", kind: "note", said: "It wraps at 720 wide", position: { x: 0, y: 300 }, created_at: at },
      { id: "legend-finding", kind: "finding", asked: "Where do the legend's colours come from?", position: { x: 340, y: 0 }, created_at: at },
      { id: "legend-draft", kind: "issue_draft", title: "The Board's legend is illegible", body: "…", state: "draft", position: { x: 680, y: 110 }, created_at: at },
      // Two addresses to read in — #1293. An Issue, and an Epic, which fills
      // the board with a node per issue and runs no scout.
      // The kind is what Fleet wrote when the address was pasted — #1394. A
      // mock has no forge, so it says what a real one would have said: which
      // host is the forge is `crates/adapters`' to know and nothing here may
      // spell one.
      { id: "legend-issue", kind: "issue", address: "https://example.invalid/o/r/issues/1293", number: "1293", title: "Read a source a person already has into a Studio", state: "open", position: { x: 0, y: 600 }, created_at: at },
      { id: "legend-milestone", kind: "epic", address: "https://example.invalid/o/r/milestone/17", number: "17", title: "Studio", position: { x: 0, y: 1400 }, created_at: at },
      // The pull request the legend's last change landed in, and a page that
      // is still a Link: an address no adapter recognised — #1394.
      { id: "legend-pull", kind: "pull_request", address: "https://example.invalid/o/r/pull/1391", number: "1391", title: "Dispatch an issue from its node, and open the Job it made", state: "merged", said: "where dispatch landed", position: { x: 340, y: 600 }, created_at: at },
      { id: "legend-page", kind: "link", address: "https://react.dev/reference/react/useId", said: "why the ids collide", position: { x: 680, y: 600 }, created_at: at },
    ],
    edges: [
      { id: "legend-e1", from: "legend-note", to: "legend-finding", kind: "produced", standing: "accepted", created_at: at },
      { id: "legend-e2", from: "legend-width", to: "legend-note", kind: "same_as", standing: "proposed", created_at: at, added_by: "helm" },
      { id: "legend-e3", from: "legend-finding", to: "legend-draft", kind: "produced", standing: "accepted", created_at: at },
    ],
  };
}

/** A Fleet keeping these Studios, answering every read and write on them. */
export function keeping(seeded: readonly Studio[] = []): StudioKeeping {
  const store = new Map<string, Studio>(seeded.map((one) => [one.id, one]));
  let fleet: FleetHandle | null = null;
  let listing: string | null = null;
  let opened: string | null = null;

  const listOf = (manifestId: string) =>
    [...store.values()].filter((one) => one.manifest_id === manifestId).reduce(foldStudio, []);

  /** What main publishes after a write it heard: the open Studio whole, and the list's row. */
  function published(studio: Studio): void {
    store.set(studio.id, studio);
    if (fleet === null) return;
    if (opened === studio.id) fleet.publish({ studio: { state: "read", studio } });
    if (listing === studio.manifest_id) {
      fleet.publish({ studios: { state: "read", manifestId: listing, list: { studios: listOf(listing) } } });
    }
  }

  function write(studioId: string, change: (studio: Studio) => Studio) {
    const held = store.get(studioId);
    if (held === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
    const studio = { ...change(held), touched_at: tick() };
    published(studio);
    return { ok: true, studio } as const;
  }

  const browsed: string[] = [];
  /**
   * Each Picture's bytes, by node, and each Sketch picture's by node and picture
   * — `frameKey`'s spelling — so a frame reads back as what was pasted.
   */
  const pictures = new Map<string, Uint8Array>();

  /**
   * A drawing as Fleet keeps it: a new picture's bytes held under a name it
   * chose, a kept one read off what the node already holds, and Fleet's two
   * refusals — nothing drawn, and a picture named that was never kept.
   */
  function sketchKept(
    nodeId: string,
    drawing: SketchToKeep,
    held: SketchDrawing | undefined,
  ): { ok: true; drawing: SketchDrawing; bytes: [string, Uint8Array][] } | { ok: false; outcome: Outcome } {
    if (drawing.boxes.length === 0 && drawing.strokes.length === 0 && drawing.pictures.length === 0) {
      return { ok: false, outcome: refusedAs("fleet.studio_node_blank", "a sketch node's `drawing` cannot be blank") };
    }
    const bytes: [string, Uint8Array][] = [];
    const kept: SketchDrawing["pictures"] = [];
    for (const { bytes: pasted, ...picture } of drawing.pictures) {
      if (pasted === undefined) {
        const was = held?.pictures.find((one) => one.id === picture.id);
        if (was === undefined) {
          return {
            ok: false,
            outcome: refusedAs(
              "fleet.studio_sketch_picture_not_kept",
              `picture \`${picture.id}\` carries no staged file, and this sketch keeps no picture by that id`,
            ),
          };
        }
        kept.push({ ...picture, frame: was.frame });
        continue;
      }
      if (pasted.byteLength > MOST_A_FRAME_MAY_WEIGH) return { ok: false, outcome: tooLarge(pasted.byteLength) };
      const filename = `${nodeId}-${mint("frame")}.png`;
      bytes.push([`${nodeId}/${picture.id}`, pasted]);
      kept.push({ ...picture, frame: { filename, byte_size: pasted.byteLength, width: picture.width, height: picture.height } });
    }
    return { ok: true, drawing: { boxes: drawing.boxes, joins: drawing.joins, strokes: drawing.strokes, pictures: kept }, bytes };
  }

  const routes = (handle: FleetHandle): StudioRoutes => {
    fleet = handle;
    return {
      watchStudios: async (manifestId) => {
        listing = manifestId;
        handle.publish({
          studios: manifestId === null ? { state: "none" } : { state: "read", manifestId, list: { studios: listOf(manifestId) } },
        });
      },
      watchStudio: async (studioId) => {
        opened = studioId;
        const studio = studioId === null ? undefined : store.get(studioId);
        handle.publish({ studio: studio === undefined ? { state: "none" } : { state: "read", studio } });
      },
      createStudio: async (manifestId) => {
        const now = tick();
        const studio: Studio = { id: mint("01STUDIO"), manifest_id: manifestId, created_at: now, touched_at: now, nodes: [], edges: [] };
        published(studio);
        return { ok: true, studio };
      },
      renameStudio: async (studioId, name) => {
        const answer = write(studioId, (studio) => ({ ...studio, name, named_by: "person" }));
        return answer.ok ? OK : answer.outcome;
      },
      pathOfFile: pathOnDisk,
      // A node by hand — #1364. **The four kinds and no more**, the way Fleet
      // refuses the rest from Bridge: a mock that took a Finding here would let
      // a test pass against a door that would not open.
      addStudioNode: async (studioId, node: StudioNodeByHand, position) => {
        // Main refuses these from a renderer: each names a file only main stages.
        if (node.kind === "picture") throw new Error("a Picture is added with its bytes, by addStudioPicture");
        if (node.kind === "sketch") throw new Error("a Sketch is added with its pictures' bytes, by addStudioSketch");
        const answer = write(studioId, (studio) => {
          const added: StudioNode = {
            ...node,
            id: mint(`${node.kind}-`),
            position,
            created_at: tick(),
            added_by: "person",
          };
          return { ...studio, nodes: [...studio.nodes, added] };
        });
        return answer.ok ? OK : answer.outcome;
      },
      // The frame is main's, and the mock has no window to take one of, so a
      // captured Note here carries everything but that.
      captureStudioNote: async (studioId, said, capture: StudioCapture) => {
        const answer = write(studioId, (studio) => {
          const y = studio.nodes.reduce((lowest, node) => Math.max(lowest, node.position.y), -260) + 260;
          const note: StudioNode = {
            id: mint("note-"),
            kind: "note",
            said,
            capture,
            position: { x: 0, y },
            created_at: tick(),
            added_by: "person",
          };
          return { ...studio, nodes: [...studio.nodes, note] };
        });
        return answer.ok ? OK : answer.outcome;
      },
      // The bytes of one Note's picture. A node that kept none is refused the
      // way Fleet refuses it, so the surface draws its sentence rather than an
      // image that never arrives.
      // Fleet's own bound and refusal, so the board says why as it would on Bridge.
      addStudioPicture: async (studioId, bytes, position) => {
        if (bytes.byteLength > MOST_A_FRAME_MAY_WEIGH) return tooLarge(bytes.byteLength);
        const id = mint("picture-");
        const size = await createImageBitmap(new Blob([bytes as BlobPart])).catch(() => null);
        const frame = {
          filename: `${id}.png`,
          byte_size: bytes.byteLength,
          width: size?.width ?? 0,
          height: size?.height ?? 0,
        };
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: [
            ...studio.nodes,
            { kind: "picture", frame, id, position, created_at: tick(), added_by: "person" } satisfies StudioNode,
          ],
        }));
        if (answer.ok) pictures.set(id, bytes);
        return answer.ok ? OK : answer.outcome;
      },
      // A Sketch drawn on the pad — 1 Oct 2026.
      addStudioSketch: async (studioId, drawing, position) => {
        const id = mint("sketch-");
        const kept = sketchKept(id, drawing, undefined);
        if (!kept.ok) return kept.outcome;
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: [
            ...studio.nodes,
            { kind: "sketch", drawing: kept.drawing, id, position, created_at: tick(), added_by: "person" } satisfies StudioNode,
          ],
        }));
        if (answer.ok) for (const [key, bytes] of kept.bytes) pictures.set(key, bytes);
        return answer.ok ? OK : answer.outcome;
      },
      saveStudioSketch: async (studioId, nodeId, drawing) => {
        const node = store.get(studioId)?.nodes.find((one) => one.id === nodeId);
        if (node?.kind !== "sketch") return refusedAs("fleet.studio_not_a_sketch", "this node is not drawn on");
        const kept = sketchKept(nodeId, drawing, node.drawing);
        if (!kept.ok) return kept.outcome;
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: studio.nodes.map((one) => (one.id === nodeId && one.kind === "sketch" ? { ...one, drawing: kept.drawing } : one)),
        }));
        if (answer.ok) for (const [key, bytes] of kept.bytes) pictures.set(key, bytes);
        return answer.ok ? OK : answer.outcome;
      },
      readStudioFrame: async (studioId, nodeId, picture) => {
        if (picture !== undefined) {
          const kept = pictures.get(`${nodeId}/${picture}`);
          return kept === undefined
            ? { ok: false, outcome: unanswered(`/studios/${studioId}/frames/${nodeId}?picture=${picture}`) }
            : { ok: true, bytes: kept, type: "image/png" };
        }
        const pasted = pictures.get(nodeId);
        if (pasted !== undefined) return { ok: true, bytes: pasted, type: "image/png" };
        const node = store.get(studioId)?.nodes.find((one) => one.id === nodeId);
        if (node?.kind !== "note" || node.capture?.frame === undefined) {
          return { ok: false, outcome: unanswered(`/studios/${studioId}/frames/${nodeId}`) };
        }
        return { ok: true, bytes: await aFrame(), type: "image/png" };
      },
      // Into the frame named, or onto the board — #1620. A Note never leaves
      // its Cluster, and a Zone holds no Zone, as Fleet refuses both.
      moveStudioNode: async (studioId, nodeId, position, within) => {
        const held = store.get(studioId)?.nodes ?? [];
        const moving = held.find((node) => node.id === nodeId);
        const was = held.find((node) => node.id === moving?.within);
        const into = held.find((node) => node.id === within);
        if (was?.kind === "cluster" && within !== was.id) {
          return refusedAs("fleet.studio_note_stays_in_its_cluster", `\`${nodeId}\` is in a Cluster, and stays in it`);
        }
        if (into !== undefined && !(into.kind === "zone" ? moving?.kind !== "zone" : into.kind === "cluster")) {
          return refusedAs("fleet.studio_frame_cannot_hold", `\`${into.id}\` is a ${into.kind} and holds no ${moving?.kind}`);
        }
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: studio.nodes.map((node) => {
            if (node.id !== nodeId) return node;
            const { within: _, ...placed } = node;
            return within === null ? { ...placed, position } : { ...placed, within, position };
          }),
        }));
        return answer.ok ? OK : answer.outcome;
      },
      // **All of them or none**, as Fleet's one write is — #1411. Every name is
      // checked before anything is dropped, so a selection carrying one node
      // this Studio does not hold leaves it whole and says so.
      removeStudioNodes: async (studioId, nodeIds) => {
        const held = store.get(studioId)?.nodes ?? [];
        const missing = nodeIds.find((wanted) => !held.some((node) => node.id === wanted));
        if (nodeIds.length === 0 || missing !== undefined) {
          return unanswered(`/studios/${studioId}/remove_nodes`);
        }
        const going = new Set(nodeIds);
        const answer = write(studioId, (studio) => ({
          ...studio,
          nodes: liftedOutOf(studio.nodes, going).filter((node) => !going.has(node.id)),
          edges: studio.edges.filter((edge) => !going.has(edge.from) && !going.has(edge.to)),
        }));
        return answer.ok ? OK : answer.outcome;
      },
      decideStudioEdge: async (studioId, edgeId, accepted) => {
        const answer = write(studioId, (studio) => ({
          ...studio,
          edges: accepted
            ? studio.edges.map((edge): StudioEdge => (edge.id === edgeId ? { ...edge, standing: "accepted" } : edge))
            : studio.edges.filter((edge) => edge.id !== edgeId),
        }));
        return answer.ok ? OK : answer.outcome;
      },
      // **Main's act and not Fleet's** — #1406: main reads the address off the
      // Studio it published and hands it to the system browser. Nothing here
      // browses, so the address is recorded and the test reads it back.
      openStudioNode: async (studioId, nodeId) => {
        const node = store.get(studioId)?.nodes.find((one) => one.id === nodeId);
        const address = node !== undefined && "address" in node ? node.address : undefined;
        if (address === undefined) return { ok: false, why: "no_address" };
        browsed.push(address);
        return { ok: true };
      },
      promoteOnStudio: async (studioId, promotion) => {
        // One Cluster at a time — the owner, 2 Oct 2026. Fleet's refusal, in its words.
        const held = store.get(studioId)?.nodes ?? [];
        const clustered =
          promotion.act === "group" && promotion.kind === "cluster"
            ? promotion.from.find((id) => {
                const note = held.find((node) => node.id === id);
                return held.some((frame) => frame.id === note?.within && frame.kind === "cluster");
              })
            : undefined;
        if (clustered !== undefined) {
          return refusedAs("fleet.studio_note_in_a_cluster", `\`${clustered}\` is already in a Cluster`);
        }
        const answer = write(studioId, (studio) => promoted(studio, promotion));
        // **A dispatch puts a row on the Board as well as a node on the
        // Studio** — #1379. A Job node holds a reference and no status, so a
        // mock that minted the node alone would draw a bare id and prove
        // nothing about the thing a person came to the Studio to do.
        if (answer.ok && promotion.act === "dispatch") atTheGate(handle, answer.studio);
        return answer.ok ? OK : answer.outcome;
      },
    };
  };

  return {
    routes,
    studios: () => [...store.values()],
    browsed: () => [...browsed],
    twoNotes: (studioId) =>
      void write(studioId, (studio) => {
        const now = tick();
        const note = (said: string, x: number): StudioNode => ({ id: mint("note-"), kind: "note", said, position: { x, y: 0 }, created_at: now });
        const first = note("The chip keeps its count after the filter is cleared", 0);
        const second = note("Overview still says three waiting after I answered one", 360);
        return { ...studio, nodes: [...studio.nodes, first, second] };
      }),
    aContradiction: (studioId) =>
      void write(studioId, (studio) => ({
        ...studio,
        nodes: [
          ...studio.nodes,
          {
            id: mint("contradiction-"),
            kind: "contradiction",
            first: "The chip reads the Board",
            second: "The chip reads its own row",
            state: "reported",
            position: { x: 0, y: 0 },
            created_at: tick(),
          },
        ],
      })),
    helmProposes: (studioId) =>
      void write(studioId, (studio) => {
        const now = tick();
        const note: StudioNode = { id: mint("note-"), kind: "note", said: "Drag me somewhere", position: { x: 0, y: 0 }, created_at: now };
        const finding: StudioNode = {
          id: mint("finding-"),
          kind: "finding",
          asked: "What does the note point at?",
          state: "proposed",
          position: { x: 360, y: 0 },
          created_at: now,
        };
        const edge: StudioEdge = { id: mint("edge-"), from: finding.id, to: note.id, kind: "answers", standing: "proposed", created_at: now, added_by: "helm" };
        return { ...studio, nodes: [...studio.nodes, note, finding], edges: [...studio.edges, edge] };
      }),
  };
}

/**
 * The Job the newest Job node references, put on the Board at the gate — the
 * dispatch gate is unchanged, and a Job from a Studio stands where every other
 * one does (#1379).
 *
 * Its title is whatever it was dispatched from: a draft's own title, or what
 * the forge calls what an address names, which is what the proposer would have
 * read it as. An address nothing has read in is titled by its address.
 */
function atTheGate(handle: FleetHandle, studio: Studio): void {
  const nodes = studio.nodes.filter((node) => node.kind === "job");
  const node = nodes[nodes.length - 1];
  if (node?.kind !== "job") return;
  const edge = studio.edges.find((one) => one.to === node.id && one.kind === "produced");
  const from = studio.nodes.find((one) => one.id === edge?.from);
  const title =
    from === undefined
      ? "Dispatched from a Studio"
      : from.kind === "issue_draft"
        ? from.title
        : "address" in from
          ? ((("title" in from ? from.title : undefined) ?? ("named" in from ? from.named : undefined)) ?? from.address)
          : "Dispatched from a Studio";
  const row = job("awaiting_approval", {
    id: node.job_id,
    handle: mint("dispatched-from-a-studio-"),
    title,
    // A person pressed Dispatch on a Studio, so the row says where from as
    // well as who — #1362.
    origin: "studio_dispatched",
    created_at: tick(),
    branch: undefined,
    assigned_drone: undefined,
  });
  handle.publish({ jobs: [...handle.state().jobs, row] });
}

/** The `studios` scenario: this repository picked, and a Fleet keeping these Studios. */
export function studying(seeded: readonly Studio[] = [legend()]): StudioFleet {
  const fleet = keeping(seeded);
  return {
    ...fleet,
    scenario: {
      ...onBoard([], { picked: repository().root }),
      name: "studios",
      says: "This repository's Studios, on a Fleet that keeps them",
      // **Approving starts it here.** A real Fleet queues a Job and a slot
      // takes it; this Fleet has no scheduler and one is always free, so
      // approving a dispatch is what a person watches turn into a run — which
      // is the half of the walkthrough a Studio's Job node is read against.
      behaves: (handle) => ({
        ...fleet.routes(handle),
        // The checkout declares what Run offers on the rail — 2 Oct 2026.
        watchCheckoutRunSheet: async (want: boolean) =>
          handle.publish({ checkoutRunSheet: want ? { state: "read", sheet: sheet() } : { state: "none" } }),
        approveDispatch: async (jobId: string) => {
          handle.publish({
            jobs: handle.state().jobs.map((one) => (one.id === jobId ? { ...one, status: "running" } : one)),
          });
          return { ok: true } as const;
        },
      }),
    },
  };
}

/**
 * The grid `everyKind()` is laid out on, wide enough for a node card and a gap. **Taller than it
 * is wide, and its top-right corner left empty**: the whiteboard fits the graph to the window and
 * draws the Proposed panel over that corner, so a node placed there is read through glass.
 *
 * The row pitch clears a Note carrying its frame, which is the tallest card there is — #1352.
 */
const place = (column: number, row: number) => ({ x: column * 340, y: row * 300 });

/** Long enough that a card cannot hold it whole: it clips, and it does not overflow — #1378. */
const LONG_ADDRESS = "https://example.invalid/armada/issues/1378#issuecomment-2847190034-and-then-some";

const MADE = "2026-09-15T11:00:00Z";
const TOUCHED = "2026-09-17T08:40:00Z";

/**
 * A Studio holding a node of every kind and an edge of every kind, proposed and accepted — what
 * `every-state` carries for Studios, the way it carries a Job in every state. `jobId` is a row on
 * that Board, so the Job node draws the state the Board holds rather than a bare id.
 */
export const EVERY_KIND_STUDIO = "01STUDIOEVERYKIND0000000000";

/** Its name, held so the Job it dispatched can be built naming the same Studio. */
export const EVERY_KIND_NAME = "Every kind of node and edge";

export function everyKind(jobId: string): Studio {
  const nodes: StudioNode[] = [
    // A Link is an address no adapter recognised — #1394 — so what stands here
    // is a documentation page, and the three forge kinds are their own rows.
    { id: "every-link", kind: "link", address: LONG_ADDRESS, said: "where the legend was drawn", position: place(0, 0), created_at: MADE },
    { id: "every-link-bare", kind: "link", address: LONG_ADDRESS, position: place(-1, 0), created_at: MADE },
    { id: "every-issue", kind: "issue", address: "https://example.invalid/armada/issues/1394", number: "1394", title: "An issue, a pull request and an epic are Links with rules bolted on", state: "open", position: place(-1, 1), created_at: MADE },
    { id: "every-pull-request", kind: "pull_request", address: "https://example.invalid/armada/pull/1391", number: "1391", title: "Dispatch an issue from its node, and open the Job it made", state: "merged", said: "where dispatch landed", position: place(-1, 2), created_at: MADE },
    { id: "every-epic", kind: "epic", address: "https://example.invalid/armada/milestone/17", number: "17", title: "Studio", read_in: { issues: 12, total: 30, took: "open", left_out: 16, kept: 2 }, position: place(-1, 3), created_at: MADE },
    // The one Note here that kept a picture — #1352. The rest draw no plate.
    { id: "every-note", kind: "note", said: "The legend under the step bar is unreadable", capture: pointedAt("every-note"), position: place(1, 0), created_at: MADE },
    { id: "every-note-wide", kind: "note", said: "It wraps at 720 wide", position: place(0, 1), created_at: MADE },
    { id: "every-note-states", kind: "note", said: "Queued and preparing read the same at a glance", position: place(1, 1), created_at: MADE },
    { id: "every-cluster", kind: "cluster", title: "The legend cannot be read", position: place(0, 2), created_at: MADE },
    {
      id: "every-run",
      kind: "run",
      run_id: "01RUNEVERYKIND000000000000",
      // Swept by retention, so the Studio kept the run whole — #1289. A Studio outlives a run's log.
      kept: {
        name: "typecheck",
        command: "pnpm typecheck",
        exit_code: 2,
        expect_exit_code: 0,
        stopped: false,
        duration_ms: 8400,
        lines: ["src/renderer/src/Board.tsx(212,9): error TS2322", "Found 1 error."],
        total_lines: 96,
        whole: false,
      },
      position: place(1, 2),
      created_at: MADE,
    },
    { id: "every-finding", kind: "finding", asked: "Where do the legend's colours come from?", position: place(2, 2), created_at: MADE },
    { id: "every-finding-asked", kind: "finding", asked: "Which states share a token?", state: "proposed", position: place(2, 3), created_at: MADE },
    {
      id: "every-contradiction",
      kind: "contradiction",
      first: "The design contract gives the legend its own row",
      second: "The Board draws it inside the step bar",
      state: "reported",
      position: place(0, 3),
      created_at: MADE,
    },
    // A Sketch written as text before 1 Oct 2026, as V85 left it: one box holding the words.
    { id: "every-sketch", kind: "sketch", drawing: { boxes: [{ id: "b1", x: 0, y: 0, body: "Legend on its own row\nunder the step bar" }], joins: [], strokes: [], pictures: [] }, position: place(1, 3), created_at: MADE },
    { id: "every-deferral", kind: "deferral", what: "Whether the legend collapses under 720", state: "open", position: place(2, 4), created_at: MADE },
    { id: "every-outline", kind: "outline", body: "Give the legend its own row\nThen fix the contrast", state: "draft", position: place(1, 4), created_at: MADE },
    { id: "every-draft", kind: "issue_draft", title: "The Board's legend is illegible", body: "…", state: "draft", position: place(0, 4), created_at: MADE },
    { id: "every-job", kind: "job", job_id: jobId, position: place(1, 5), created_at: MADE },
  ];
  // `produced` is drawn by the Studio and never proposed (`docs/concepts/studio.md`, Edges), so
  // only the three relations are here twice, once waiting on a person and once decided.
  const edges: StudioEdge[] = ([
    ["every-produced-note", "every-link", "every-note", "produced", "accepted"],
    ["every-produced-contradiction", "every-link", "every-contradiction", "produced", "accepted"],
    ["every-produced-capture", "every-run", "every-note-states", "produced", "accepted"],
    ["every-produced-finding", "every-note", "every-finding", "produced", "accepted"],
    ["every-produced-cluster", "every-note-wide", "every-cluster", "produced", "accepted"],
    ["every-produced-outline", "every-finding", "every-outline", "produced", "accepted"],
    ["every-produced-draft", "every-outline", "every-draft", "produced", "accepted"],
    ["every-produced-sketch", "every-sketch", "every-draft", "produced", "accepted"],
    ["every-produced-job", "every-draft", "every-job", "produced", "accepted"],
    ["every-same-as", "every-note", "every-note-wide", "same_as", "accepted"],
    ["every-same-as-asked", "every-note", "every-note-states", "same_as", "proposed"],
    ["every-blocks", "every-deferral", "every-outline", "blocks", "accepted"],
    ["every-blocks-asked", "every-contradiction", "every-draft", "blocks", "proposed"],
    ["every-answers", "every-finding", "every-deferral", "answers", "accepted"],
    ["every-answers-asked", "every-finding-asked", "every-deferral", "answers", "proposed"],
  ] as const).map(([id, from, to, kind, standing]) => ({ id, from, to, kind, standing, created_at: MADE }));
  return {
    id: EVERY_KIND_STUDIO,
    manifest_id: MANIFEST_ID,
    name: EVERY_KIND_NAME,
    created_at: MADE,
    touched_at: TOUCHED,
    nodes,
    edges,
  };
}

/** A Studio nobody has named, so the list draws what an untitled one is called. */
export function untitled(): Studio {
  const at = "2026-09-16T17:05:00Z";
  return {
    id: "01STUDIOUNTITLED00000000000",
    manifest_id: MANIFEST_ID,
    created_at: at,
    touched_at: at,
    nodes: [
      { id: "untitled-link", kind: "link", address: "https://example.invalid/armada/docs/concepts/studio.md#notes", said: "what a Note is, and what fixes it", position: place(0, 0), created_at: at },
      { id: "untitled-note", kind: "note", said: "Capture on another repository's web app waits on a security review", position: place(0, 1), created_at: at },
    ],
    edges: [{ id: "untitled-produced", from: "untitled-link", to: "untitled-note", kind: "produced", standing: "accepted", created_at: at }],
  };
}

/**
 * Every issue the milestone holds. Each one's number, title and state are
 * fields: the read already answered all three, so nothing is left for a later
 * fetch — #1394.
 */
const A_MILESTONES_ISSUES: Extract<StudioNodeContent, { kind: "issue" }>[] = [
  { kind: "issue", address: "https://example.invalid/o/r/issues/1293", number: "1293", title: "An issue cannot be read into a Studio", state: "open" },
  { kind: "issue", address: "https://example.invalid/o/r/issues/1291", number: "1291", title: "Promotion: cluster, defer, write up", state: "closed" },
  { kind: "issue", address: "https://example.invalid/o/r/issues/1275", number: "1275", title: "Kit manages connections", state: "open" },
];

/**
 * An Epic read in, with the answer the person gave — #1405. **Reading it in
 * again narrows or widens what is here**: it makes what is missing, takes back
 * the Issue nodes it made that the answer no longer wants, and leaves standing
 * any that something hangs off, which is the rule Fleet holds.
 */
function epicReadIn(studio: Studio, nodeId: string, position: { x: number; y: number }, take: EpicTake): Studio {
  const wanted = A_MILESTONES_ISSUES.filter((issue) => take === "everything" || issue.state === "open");
  // Its issues in one Zone, made on its first read and filled after — #1620.
  const held = studio.nodes.find(
    (node) => node.kind === "zone" && studio.edges.some((edge) => edge.kind === "produced" && edge.from === nodeId && edge.to === node.id),
  );
  const zoned = held === undefined ? made(studio, { kind: "zone" }, [nodeId], position) : studio;
  const zone = held?.id ?? zoned.nodes[zoned.nodes.length - 1]!.id;
  const down = (n: number) => ({ x: INSET, y: HEAD + n * 180 });
  studio = zoned;
  const addressOf = (node: StudioNode) => ("address" in node ? node.address : "");
  const mine = studio.nodes.filter(
    (node) => node.kind === "issue" && studio.edges.some((edge) => edge.kind === "produced" && edge.from === nodeId && edge.to === node.id),
  );
  const keeps = (node: StudioNode) => studio.edges.some((edge) => edge.from === node.id || (edge.to === node.id && edge.from !== nodeId));
  const standing = mine.filter((node) => wanted.some((issue) => issue.address === addressOf(node)) || keeps(node));
  const gone = mine.filter((node) => !standing.includes(node)).map((node) => node.id);
  const missing = wanted.filter((issue) => !standing.some((node) => addressOf(node) === issue.address));
  const narrowed: Studio = {
    ...studio,
    nodes: studio.nodes.filter((node) => !gone.includes(node.id)),
    edges: studio.edges.filter((edge) => !gone.includes(edge.from) && !gone.includes(edge.to)),
  };
  const filled = missing.reduce((so_far, issue, n) => made(so_far, issue, [nodeId], down(standing.length + n), zone), narrowed);
  const read = {
    issues: standing.length + missing.length,
    total: A_MILESTONES_ISSUES.length,
    took: take,
    left_out: A_MILESTONES_ISSUES.length - wanted.length,
    kept: standing.filter((node) => !wanted.some((issue) => issue.address === addressOf(node))).length,
  };
  return {
    ...filled,
    nodes: filled.nodes.map((node) => (node.id === nodeId && node.kind === "epic" ? { ...node, read_in: read } : node)),
  };
}

/** Fleet's `framing::INSET` and `HEAD`: room inside a frame's edge, and above what it holds for its head. */
const INSET = 24;
const HEAD = 48;

/** Where a node sits on the board itself, through every frame it is inside. */
function onTheBoard(nodes: readonly StudioNode[], id: string | undefined): { x: number; y: number } {
  const node = nodes.find((one) => one.id === id);
  if (node === undefined) return { x: 0, y: 0 };
  const corner = node.within === undefined ? { x: 0, y: 0 } : onTheBoard(nodes, node.within);
  return { x: node.position.x + corner.x, y: node.position.y + corner.y };
}

/** What the frames going held, lifted onto whatever held each, where they were on the board — Fleet's rule. */
function liftedOutOf(nodes: readonly StudioNode[], going: ReadonlySet<string>): StudioNode[] {
  return nodes.map((node) => {
    let within = node.within;
    while (within !== undefined && going.has(within)) within = nodes.find((one) => one.id === within)?.within;
    if (within === node.within) return node;
    const at = onTheBoard(nodes, node.id);
    const corner = within === undefined ? { x: 0, y: 0 } : onTheBoard(nodes, within);
    const { within: _, ...placed } = node;
    const position = { x: at.x - corner.x, y: at.y - corner.y };
    return within === undefined ? { ...placed, position } : { ...placed, within, position };
  });
}

/**
 * A Cluster drawn round the Notes it is made of, as Fleet's `framing::around`
 * places it: inside the frame they share, or on the board, with each Note
 * where it was on the board.
 */
function framedRound(studio: Studio, cluster: StudioNode, from: readonly string[]): Studio {
  const members = studio.nodes.filter((node) => from.includes(node.id));
  const first = members[0]?.within;
  const within = members.every((node) => node.within === first) ? first : undefined;
  const at = (node: StudioNode) => (within === undefined ? onTheBoard(studio.nodes, node.id) : node.position);
  const corner = {
    x: Math.min(...members.map((node) => at(node).x)) - INSET,
    y: Math.min(...members.map((node) => at(node).y)) - HEAD,
  };
  const { within: _, ...bare } = cluster;
  const placed: StudioNode = within === undefined ? { ...bare, position: corner } : { ...bare, within, position: corner };
  return {
    ...studio,
    nodes: studio.nodes.map((node) => {
      if (node.id === cluster.id) return placed;
      if (!from.includes(node.id)) return node;
      const spot = at(node);
      return { ...node, within: cluster.id, position: { x: spot.x - corner.x, y: spot.y - corner.y } };
    }),
  };
}

/** A node the way Fleet writes one, with a `produced` edge from each node that made it. */
function made(
  studio: Studio,
  content: StudioNodeContent,
  from: readonly string[],
  position: { x: number; y: number },
  within?: string,
): Studio {
  const now = tick();
  const node = {
    ...content,
    id: mint("node-"),
    ...(within === undefined ? {} : { within }),
    position,
    created_at: now,
    added_by: "person",
  } as StudioNode;
  const edges = from.map(
    (source): StudioEdge => ({ id: mint("edge-"), from: source, to: node.id, kind: "produced", standing: "accepted", created_at: now }),
  );
  return { ...studio, nodes: [...studio.nodes, node], edges: [...studio.edges, ...edges] };
}

/** A Contradiction the rung ended as it went. Every other kind is left as it was. */
function ended(studio: Studio, nodeId: string, outcome: string): Studio {
  return {
    ...studio,
    nodes: studio.nodes.map((node) =>
      node.id === nodeId && node.kind === "contradiction" && node.state === "reported" ? { ...node, state: outcome } : node,
    ),
  };
}

/** One rung, as Fleet writes it. Every refusal is Fleet's own and none of them is here. */
function promoted(studio: Studio, promotion: StudioPromotion): Studio {
  switch (promotion.act) {
    case "group": {
      const content: StudioNodeContent =
        promotion.kind === "cluster" ? { kind: "cluster", title: promotion.title } : { kind: "outline", body: promotion.body };
      const grouped = made(studio, content, promotion.from, promotion.position);
      // A Cluster is drawn round its Notes, wherever the request said — #1620.
      if (promotion.kind !== "cluster") return grouped;
      return framedRound(grouped, grouped.nodes[grouped.nodes.length - 1]!, promotion.from);
    }
    case "defer": {
      const with_it = made(studio, { kind: "deferral", what: promotion.what }, [promotion.raised_on], promotion.position);
      const deferral = with_it.nodes[with_it.nodes.length - 1]!;
      const blocks: StudioEdge[] =
        promotion.blocks === undefined
          ? []
          : [{ id: mint("edge-"), from: deferral.id, to: promotion.blocks, kind: "blocks", standing: "accepted", created_at: tick() }];
      return ended({ ...with_it, edges: [...with_it.edges, ...blocks] }, promotion.raised_on, "deferral");
    }
    case "write_up": {
      const draft: StudioNodeContent = { kind: "issue_draft", title: promotion.title, body: promotion.body };
      return ended(made(studio, draft, [promotion.node_id], promotion.position), promotion.node_id, "issue_draft");
    }
    case "edit_draft":
      return {
        ...studio,
        nodes: studio.nodes.map((node) =>
          node.id === promotion.node_id && node.kind === "issue_draft" ? { ...node, title: promotion.title, body: promotion.body } : node,
        ),
      };
    case "edit_link": {
      // The address is read off the node and never off the request, and a blank line clears it.
      const said = promotion.said.trim() === "" ? undefined : promotion.said.trim();
      const line = (node: StudioNode) => (node.id === promotion.node_id && node.kind === "link" ? { ...node, said } : node);
      return { ...studio, nodes: studio.nodes.map(line) };
    }
    case "settle":
      return {
        ...studio,
        nodes: studio.nodes.map((node) =>
          node.id === promotion.node_id && node.kind === "contradiction"
            ? { ...node, state: promotion.outcome, ...(promotion.outcome === "resolved_here" ? { answer: promotion.answer } : {}) }
            : node,
        ),
      };
    case "dispatch":
      return made(studio, { kind: "job", job_id: mint("01JOB") }, [promotion.node_id], promotion.position);
    case "read_in":
      return readIn(studio, promotion.node_id, promotion.position, promotion.take ?? "everything");
  }
}

/** An address whose scout reads it and asks for nothing — what Fleet makes of an empty answer. */
export const READS_AS_NOTHING = "https://example.invalid/o/r/wiki/Glossary";

/** What Fleet's one Note says when a read-in finds nothing — `NOTHING_FOUND` in `crates/fleet/src/reading_in.rs`. */
export const NOTHING_FOUND = "Nothing was found that could be pulled into the studio.";

/**
 * Every read-in still reading, ended answered with nothing to place: Fleet lands one Note beside
 * its Finding, in its Zone, produced by the Finding and by the source the way every read-in node is.
 */
function answeredWithNothing(studio: Studio): Studio {
  let out = studio;
  for (const node of studio.nodes) {
    if (node.kind !== "finding" || node.state !== "gathering") continue;
    const { state: _reading, ...rest } = node;
    const ended: StudioNode = { ...rest, learned: "Nothing in the page bears on this repository.", ended: { outcome: "answered", cost_micros: 900 } };
    const source = studio.edges.find((edge) => edge.to === node.id && edge.kind === "produced")?.from;
    const at = { x: node.position.x + 340, y: node.position.y };
    out = made(
      { ...out, nodes: out.nodes.map((one) => (one.id === node.id ? ended : one)) },
      { kind: "note", said: NOTHING_FOUND },
      [node.id, ...(source === undefined ? [] : [source])],
      at,
      node.within,
    );
  }
  return out;
}

/**
 * An address read in — #1293, #1394. **What Fleet fetched is decided here by
 * the address**, since a mock has no network: an Epic fills in as one Issue per
 * issue with no scout, and every other source leaves an ended Finding beside
 * the Notes and the Contradiction its scout asked for.
 */
function readIn(studio: Studio, nodeId: string, position: { x: number; y: number }, take: EpicTake): Studio {
  const link = studio.nodes.find((node) => node.id === nodeId);
  if (link === undefined || !("address" in link)) return studio;
  if (link.kind === "epic") return epicReadIn(studio, nodeId, position, take);
  // **Everything it brings back lands in one Zone**, the Finding first and
  // each Cluster round its Notes — the owner, 2 Oct 2026, as Fleet lays it out.
  const zoned = made(studio, { kind: "zone" }, [nodeId], position);
  const zone = zoned.nodes[zoned.nodes.length - 1]!.id;
  const down = (n: number) => ({ x: INSET, y: HEAD + n * 180 });
  const finding: StudioNodeContent = {
    kind: "finding",
    asked: `Read in ${link.address}`,
    checkout: { commit: "4bdb169c2f", uncommitted: false },
    sources: [{ address: link.address, kind: "issue", cut: 0 }],
    read: ["docs/concepts/studio.md"],
    searched: ["Workspace in docs"],
    learned: "Two claims, and one of them disagrees with the checkout.",
    ended: { outcome: "answered", cost_micros: 3_100 },
  };
  if (link.address === READS_AS_NOTHING) {
    // Still reading: no `learned` and no `ended` yet, and its state says so.
    const { learned: _, ended: __, ...asked } = finding as Extract<StudioNodeContent, { kind: "finding" }>;
    const reading = made(zoned, asked, [nodeId], down(0), zone);
    const gathering = { ...reading, nodes: reading.nodes.map((node, at) => (node.kind === "finding" && at === reading.nodes.length - 1 ? { ...node, state: "gathering" } : node)) };
    // Answered at once: a walk frames the board after it, so nothing waits on a timer.
    return answeredWithNothing(gathering);
  }
  // Ended, so no state: `frozen` went on 1 Oct 2026.
  const marked = made(zoned, finding, [nodeId], down(0), zone);
  const clustered = made(marked, { kind: "cluster", title: "What to read in first" }, [nodeId], { x: INSET + 340, y: HEAD }, zone);
  const cluster = clustered.nodes[clustered.nodes.length - 1]!.id;
  const noted = made(clustered, { kind: "note", said: "The issue wants Links read in as a second, refusable step" }, [nodeId], down(0), cluster);
  const twice = made(noted, { kind: "note", said: "It names the forge, web pages, sessions and Helm threads as the first sources" }, [nodeId], down(1), cluster);
  const contradicted = made(
    twice,
    {
      kind: "contradiction",
      first: "The issue says Connections have no home yet",
      second: "docs/concepts/kit.md already gives them one",
    },
    [nodeId],
    { x: INSET + 680, y: HEAD },
    zone,
  );
  const first = noted.nodes[noted.nodes.length - 1]!;
  const note = twice.nodes[twice.nodes.length - 1]!;
  const contradiction = contradicted.nodes[contradicted.nodes.length - 1]!;
  const member = (from: string): StudioEdge => ({ id: mint("edge-"), from, to: cluster, kind: "produced", standing: "accepted", created_at: tick() });
  return {
    ...contradicted,
    edges: [
      ...contradicted.edges,
      member(first.id),
      member(note.id),
      { id: mint("edge-"), from: note.id, to: contradiction.id, kind: "blocks", standing: "proposed", created_at: tick(), added_by: "helm" },
    ],
  };
}
