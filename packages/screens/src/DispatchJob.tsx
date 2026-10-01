// Dispatch a job: describe the work, and that is the whole of it.
//
// **One surface and one way through it.** The Job proposer reads the request
// and answers the title, the workflow and, where the work is several jobs, the
// order between them. The hand form this used to swap to is gone — the owner's
// call of 2026-09-23, once the Settings block carried every decision it held.
//
// **The press leaves, and this screen holds no proposal at all.** The wait and
// the answer are `ProposalPage`'s — the owner's decision of 30 Sep 2026, *the
// wait is a destination*.

// **The double-press guard is still a ref.** Two presses are two model calls
// and two drafted plans, and nothing on the preload call guards it. Leaving the
// surface is most of the fix; the ref is the rest, because unmounting is a
// render — a key repeat and a synthetic click both reach the handler in the
// same task, with the button still in the document.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { DispatchRequest, DispatchSettings, SketchPad } from "@armada/components";
import type {
  DispatchSettingsValue,
  Refs,
  RequestMode,
  SketchBox,
  SketchLine,
  SketchStroke,
} from "@armada/components";
import type { LeftOutWorkflow, StagedAttachment, WorkflowSummary } from "@armada/protocol";

import { howManySet } from "./draft/dispatch";
import type { DispatchSettingsView } from "./draft/dispatch";
import type { BranchesAnswer } from "./draft/branches";
import type { LandingRule } from "./draft/landing";
import {
  drawingOf,
  isDrawn,
  nextShapeId,
  withBody,
  withJoin,
  withPlace,
  withShape,
  withStroke,
  withoutLastStroke,
  withoutShapes,
} from "./draft/sketch";
import type { Drawing, SketchAttachment } from "./draft/sketch";

/**
 * What the sketch's chip is called. **Numbered because a request may carry
 * more than one picture**, even though the composer holds one today (#1547).
 */
const SKETCH_NAME = "sketch 1";

/** What the pad is, read to somebody who cannot see it. */
const PAD_LABEL = "The picture attached to this request";

export type DispatchJobProps = {
  /**
   * Send the request, with everything staged against it.
   *
   * **It answers nothing, because the caller leaves this surface on the press**
   * and the wait is `ProposalPage`'s. Called at most once per mount, and never
   * with a blank request.
   */
  onPropose: (request: string, attachments: readonly StagedAttachment[]) => void;
  /**
   * Put a picked or pasted file somewhere the Job can name, and answer with
   * the path. The same call `Composer`'s `onStage` makes; this screen owns
   * the staged list until `onPropose` carries it on.
   */
  onStage: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{ path: string }>;
  /** Narrow the checkout against typed text, for the `@` mention popup. */
  onSearchFiles: (query: string) => Promise<readonly string[]>;
  /**
   * The way out of the composer, drawn on the head of the card that asks for
   * the request. The caller's control; absent draws none.
   */
  close?: ReactNode;
  /** The repository this dispatch is for. A fact, answered before this opens. */
  repository?: string;
  /**
   * What the request field opens on. **Absent opens it empty**, which is every
   * dispatch a person starts themselves; a moment being replayed hands one in.
   */
  opensOn?: string;
  /**
   * Where the work starts and where it lands, as the Manifest declares them.
   * Both `null` is a Manifest naming no base, which draws empty fields rather
   * than a branch name nobody chose.
   */
  landing: LandingRule;
  /**
   * The repository's branches, for the two ref fields to pick over. **`null`
   * is nothing having listed them**, which is Bridge on a real Fleet — no
   * operation asks a repository for its refs, so the fields draw as the plain
   * ones they were. `draft/branches.ts` names the read it waits on.
   */
  branches: BranchesAnswer;
  /** The workflows Fleet holds, for the settings block's override. */
  workflows: readonly WorkflowSummary[];
  /**
   * The Kit and carried definitions Fleet runs without, each with why — #425.
   * Drawn under the Settings block's Workflow field, which is where a workflow
   * is picked now that the hand form is gone.
   */
  leftOut?: readonly LeftOutWorkflow[];
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** How many Drones this machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
  /** What the settings block opens on. Absent is nothing set, which is the ordinary case. */
  settings?: DispatchSettingsView;
  /**
   * The picture this opens holding, where a moment carries one. **Absent opens
   * Sketch on a blank pad**, which is every dispatch somebody starts here.
   *
   * **Nothing stages it yet.** The wire takes a staged path and a filename, so
   * what goes out with the request is unchanged until #1545 promotes the shape;
   * the pad is the surface, and staging the PNG is that pull request's.
   */
  sketch?: SketchAttachment;
  /**
   * Whether the request field or its attachments hold anything closing would
   * throw away. Reported as it changes, and `false` on the way out, so the
   * caller's way out can ask first. #1366.
   */
  onTyped?: (typed: boolean) => void;
  /** Nothing may be dispatched while the connection is not live. */
  disabled: boolean;
  /** Why the controls are off, where they are. */
  disabledNote?: ReactNode;
};

export function DispatchJob({
  onPropose,
  onStage,
  onSearchFiles,
  close,
  repository,
  opensOn,
  landing,
  branches,
  workflows,
  leftOut,
  models,
  machineCap,
  settings: settingsOpenOn,
  sketch,
  onTyped,
  disabled,
  disabledNote,
}: DispatchJobProps) {
  const [request, setRequest] = useState(opensOn ?? "");
  const [attachments, setAttachments] = useState<StagedAttachment[]>([]);
  // The two refs, seeded from the Manifest and a person's to change. Held as
  // strings rather than as the rule's `string | null`: a field's empty value is
  // "", and `null` here would be a second spelling of the same emptiness.
  const [refs, setRefs] = useState<Refs>({
    from: landing.from_ref ?? "",
    target: landing.target ?? "",
  });
  const [links, setLinks] = useState<string[]>([]);
  const [settings, setSettings] = useState<DispatchSettingsValue>(asChosen(settingsOpenOn));
  const [settingsOpen, setSettingsOpen] = useState(false);
  // Words or a picture. Held here rather than in the card, because both halves
  // of one request outlive a switch between them.
  const [mode, setMode] = useState<RequestMode>("write");
  const [drawing, setDrawing] = useState<Drawing>(() => drawingOf(sketch));
  const [said, setSaid] = useState(sketch?.said ?? "");
  // One request per mount. A ref rather than state because nothing renders from
  // it: it is the guard, not a reading, and the unmount the press causes is a
  // render, so two presses in one task both find the button in the document.
  const sent = useRef(false);

  // What a close would lose here: the words in the field, anything staged or
  // linked against them, and any setting moved off what it opened on.
  const typed =
    request.trim() !== "" ||
    attachments.length > 0 ||
    links.length > 0 ||
    isDrawn(drawing) ||
    howManySet(asDrafted(settings)) > 0;
  useEffect(() => {
    onTyped?.(typed);
    return () => onTyped?.(false);
  }, [typed, onTyped]);

  function dispatch(): void {
    // Links go out with the request, one to a line. The proposer's field is
    // prose and a ticket address in it is what it already reads; a chip is so
    // the person can see and take back what they pasted.
    const asked = [request.trim(), ...links].join("\n");
    if (sent.current || disabled || request.trim() === "") return;
    sent.current = true;
    onPropose(asked, attachments);
  }

  return (
    <DispatchRequest
      request={request}
      onRequest={setRequest}
      {...(repository === undefined ? {} : { repository })}
      refs={refs}
      onRefs={setRefs}
      // Handed straight over: a `BranchView` and the control's own
      // `BranchOption` are the same three fields under the same names, so a
      // mapper here would be the seam `asChosen` is without the rename that
      // earns one.
      branches={branches}
      links={links}
      onAddLink={(address) =>
        setLinks((current) => (current.includes(address) ? current : [...current, address]))
      }
      onRemoveLink={(address) => setLinks((current) => current.filter((one) => one !== address))}
      settings={
        <DispatchSettings
          open={settingsOpen}
          onOpenChange={setSettingsOpen}
          settings={settings}
          onSettings={setSettings}
          workflows={workflows}
          {...(leftOut === undefined ? {} : { leftOut })}
          models={models}
          machineCap={machineCap}
          disabled={disabled}
        />
      }
      mode={mode}
      onMode={setMode}
      sketchPad={
        <SketchPad
          label={PAD_LABEL}
          boxes={drawing.shapes.map(asBox)}
          lines={drawing.joins.map(asLine)}
          strokes={drawing.strokes.map(asStroke)}
          said={said}
          onSaid={setSaid}
          {...(sketch?.produced_by === undefined ? {} : { from: sketch.produced_by })}
          onAdd={(at) =>
            setDrawing((one) =>
              withShape(one, { id: nextShapeId(one), x: at.x, y: at.y, body: "" }),
            )
          }
          onBody={(id, body) => setDrawing((one) => withBody(one, id, body))}
          onMove={(id, at) => setDrawing((one) => withPlace(one, id, at))}
          onRemove={(ids) => setDrawing((one) => withoutShapes(one, ids))}
          onJoin={(from, to) => setDrawing((one) => withJoin(one, from, to))}
          onDraw={(points) => setDrawing((one) => withStroke(one, points))}
          onUndo={() => setDrawing(withoutLastStroke)}
          disabled={disabled}
        />
      }
      // Nothing drawn is nothing attached: an empty pad is a mode somebody
      // opened, not a picture.
      {...(isDrawn(drawing)
        ? {
            // **The name alone.** Where the picture was made is the pad's line,
            // which names the node as well — the chip carrying `From a Studio`
            // too said `a Studio` twice on one screen (the owner, 28 Sep 2026).
            sketch: { name: SKETCH_NAME },
          }
        : {})}
      attachments={attachments}
      onStage={onStage}
      onSearchFiles={onSearchFiles}
      onAttach={(attachment) => setAttachments((current) => [...current, attachment])}
      onRemoveAttachment={(path) =>
        setAttachments((current) => current.filter((attachment) => attachment.path !== path))
      }
      onDispatch={dispatch}
      close={close}
      disabled={disabled}
      disabledNote={disabledNote}
    />
  );
}

/**
 * The draft's settings as the control holds them, and back again.
 *
 * **Two spellings of four values, and the seam is here on purpose.** The draft
 * is wire-shaped because that is where it is going (`draft/dispatch.ts` names
 * the module); a component's props are the app's own. Mapping in one function
 * each is what keeps a rename on either side a compile error rather than a
 * field that silently stops arriving.
 */
function asChosen(view: DispatchSettingsView = {}): DispatchSettingsValue {
  const chosen: DispatchSettingsValue = {};
  if (view.workflow_id !== undefined) chosen.workflowId = view.workflow_id;
  if (view.tiers !== undefined) chosen.tiers = { ...view.tiers };
  if (view.drone_cap !== undefined) chosen.droneCap = view.drone_cap;
  if (view.lands !== undefined) chosen.lands = view.lands;
  return chosen;
}

function asDrafted(chosen: DispatchSettingsValue): DispatchSettingsView {
  const view: DispatchSettingsView = {};
  if (chosen.workflowId !== undefined) view.workflow_id = chosen.workflowId;
  if (chosen.tiers !== undefined) view.tiers = { ...chosen.tiers };
  if (chosen.droneCap !== undefined) view.drone_cap = chosen.droneCap;
  if (chosen.lands !== undefined) view.lands = chosen.lands;
  return view;
}

/**
 * The draft's boxes, joins and strokes as the pad holds them, and nothing else.
 *
 * **Two spellings of one shape, mapped in one function each** — the same seam
 * `asChosen` is, and for the same reason: the draft is wire-shaped because
 * that is where it is going, and a component's props are the app's own.
 */
function asBox(shape: Drawing["shapes"][number]): SketchBox {
  return { id: shape.id, x: shape.x, y: shape.y, body: shape.body };
}

function asLine(join: Drawing["joins"][number]): SketchLine {
  return { id: join.id, from: join.from, to: join.to };
}

function asStroke(stroke: Drawing["strokes"][number]): SketchStroke {
  return { id: stroke.id, points: stroke.points };
}
