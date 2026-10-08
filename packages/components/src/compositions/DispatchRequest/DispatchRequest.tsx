import type { ChangeEvent, ClipboardEvent, FocusEvent, ReactNode } from "react";
import { useRef, useState } from "react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Input } from "../../primitives/Input/Input";
import { BranchPicker } from "../BranchPicker/BranchPicker";
import type { BranchOption } from "../BranchPicker/BranchPicker";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_DISPATCH } from "../../guides";
import { MentionPopover, useMention } from "../../primitives/MentionPopover/MentionPopover";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import type { StagedAttachment } from "@armada/protocol";

/**
 * Dispatch a job by describing the work. One field, one press, and the Job
 * proposer decides the rest.
 *
 * **Describing the work is the only way in.** A person types what they want
 * done, or pastes a link; they pick no workflow and write no title, and
 * `../../../../../docs/concepts/job-proposer.md` says why. The Settings block
 * is where a workflow, a model or a cap is overridden instead.
 *
 * **The press leaves this surface.** A dispatched request is a Job from the
 * press — `job-statuses.toml`, `proposing` — so the wait is that Job's row on
 * the Board and the reading is `ProposerWait` inside its lead. Nothing here
 * draws a proposal, and the one control never goes pending: by the time Fleet
 * answers, this card is gone.
 */
export type DispatchRequestProps = {
  /**
   * What is typed. Controlled by the caller, because a moment being replayed
   * opens this card on words somebody already wrote.
   */
  request: string;
  onRequest: (request: string) => void;
  /**
   * The repository this would be dispatched into. A fact rather than a field —
   * Bridge dispatches into the workspace it is pointed at, and the ask upstream
   * of this surface is where one is chosen. Absent draws no row.
   */
  repository?: string;
  /**
   * Where the work starts, and where it lands.
   *
   * **Two refs and not one** (#1530, 22 Sep). They differ when you start from
   * an unmerged branch or land in a long-lived one, and neither is drawn as
   * the other's default — a form saying "lands in main (from main)" would make
   * the second field look like a copy of the first rather than a choice.
   */
  refs: Refs;
  onRefs: (refs: Refs) => void;
  /**
   * The repository's branches, for the two fields above to pick over.
   *
   * **`null` is nothing having listed them**, which draws the two plain fields
   * they were — an empty picker would say the repository has no branches, and
   * nothing has asked it. Nothing on the wire answers this today; the shape is
   * `packages/screens/src/draft/branches.ts`.
   */
  branches?: readonly BranchOption[] | null;
  /**
   * Addresses attached beside the request — a ticket, a pull request, a page.
   *
   * **They go out with the request**, one per line under what was typed,
   * because the proposer's own field is prose and a link in it is what it
   * already reads. The chips are so that a person can see and take back what
   * they pasted, which a line buried in a paragraph does not allow.
   */
  links?: readonly string[];
  onAddLink?: (address: string) => void;
  onRemoveLink?: (address: string) => void;
  /**
   * The Studio node this request came off, where it came off one. Read-only:
   * it is provenance, and it is taken back by leaving the surface.
   */
  node?: { name: string };
  /**
   * The optional Settings block, built by the caller. **Absent draws none** —
   * a surface with nothing to set is not a surface with an empty block on it.
   */
  settings?: ReactNode;
  /**
   * Which of the two ways of saying it is open, and the surface Sketch draws.
   *
   * **Absent draws no switch at all**, which is a composer that takes words
   * only. Where one is given, `mode` is controlled for the reason `request` is:
   * what a person drew outlives a swap back to Write, so the caller holds it.
   */
  mode?: RequestMode;
  onMode?: (mode: RequestMode) => void;
  sketchPad?: ReactNode;
  /**
   * The sketch, as the chip beside the prompt reads it. **Absent attaches
   * none** — an empty pad is not an attachment.
   *
   * **The name and nothing else.** Where the picture was made is the pad's own
   * line, which names the Studio node as well; a chip saying `From a Studio`
   * beside it put `a Studio` on the screen twice, and the owner read the two
   * on 28 Sep 2026 and asked what the difference was.
   */
  sketch?: { name: string };
  /**
   * Narrow the checkout against typed text, for the `@` mention popup —
   * `crate::files::search` on the other side of the wire. Never rejects: a
   * call that could not be made answers empty, the way `JobCommands.searchFiles`
   * does, so a popup nobody may even have open never raises a toast.
   */
  onSearchFiles: (query: string) => Promise<readonly string[]>;
  /**
   * Files pasted or picked against the request, before the Job it will
   * attach to exists. Controlled, the way `request` is — this draws the chips
   * and the picker, and the caller carries what comes back on `onDispatch`.
   */
  attachments: readonly StagedAttachment[];
  /**
   * Put a picked or pasted file somewhere the Job can name, and answer with
   * the path. Staged before any Job exists, so there is no id to key it on —
   * the same call `Composer` makes through `onStage`.
   */
  onStage: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{ path: string }>;
  /** One file staged, appended to `attachments`. */
  onAttach: (attachment: StagedAttachment) => void;
  /** Take a staged file back, by the path `onStage` answered with. */
  onRemoveAttachment: (path: string) => void;
  /**
   * Send it. Never called with a blank request — the control is off until one.
   *
   * **The caller leaves this surface on the press**, so nothing here waits for
   * an answer and this returns nothing to wait on.
   */
  onDispatch: () => void;
  /**
   * The way out of the surface this card belongs to, drawn at the head's
   * trailing edge — the card is what a person is leaving, so the exit sits on
   * it rather than loose above it. The caller's control; absent draws none.
   */
  close?: ReactNode;
  /** Nothing may be dispatched while the connection is not live. */
  disabled?: boolean;
  /** Why the controls are off, where they are. A dead control with no reason reads as broken. */
  disabledNote?: ReactNode;
  /** The Request field takes focus on mount, the caret after what it opens holding. */
  focused?: boolean;
};

/**
 * Where the work starts, and where it lands. Empty is a ref nobody has named —
 * a Manifest declaring no base, which is drawn as an empty field rather than
 * as a branch name this surface made up.
 */
export type Refs = { from: string; target: string };

/**
 * The two ways of saying what you want. **Words or a picture, never both at
 * once on screen** — the prompt and the pad are one request, and a card
 * holding the two side by side at 768px would give neither room to be read.
 */
export type RequestMode = "write" | "sketch";

/** What the switch's two segments are called. */
const MODES = [
  { id: "write", label: "Write" },
  { id: "sketch", label: "Sketch" },
];

/** What the field asks for, and the two things it takes. */
export const REQUEST_PLACEHOLDER = "Describe the work, or paste a link to a ticket.";
const PLACEHOLDER = REQUEST_PLACEHOLDER;

export function DispatchRequest({
  request,
  onRequest,
  repository,
  refs,
  onRefs,
  branches = null,
  links = [],
  onAddLink,
  onRemoveLink,
  node,
  settings,
  mode = "write",
  onMode,
  sketchPad,
  sketch,
  onSearchFiles,
  attachments,
  onStage,
  onAttach,
  onRemoveAttachment,
  onDispatch,
  close,
  disabled = false,
  disabledNote,
  focused = false,
}: DispatchRequestProps) {
  const empty = request.trim() === "";
  // The hidden file input the "Attach" button clicks through. A ref rather
  // than state because nothing here reads its value; `onChange` does.
  const fileInputRef = useRef<HTMLInputElement>(null);
  const mention = useMention(request, onRequest, onSearchFiles);

  /**
   * One file staged and handed to `onAttach`. Shared by the picker and a
   * pasted screenshot — both hand this the same three facts and differ only
   * in where the bytes came from. Mirrors `Composer`'s own `stage()`.
   */
  async function stage(file: File): Promise<void> {
    const bytes = await file.arrayBuffer();
    const { path } = await onStage(bytes, file.name, file.type);
    onAttach({ path, filename: file.name, mimeType: file.type });
  }

  function onFilesPicked(event: ChangeEvent<HTMLInputElement>): void {
    const files = event.target.files;
    if (files !== null) for (const file of Array.from(files)) void stage(file);
    // Cleared so picking the same file again still fires `onChange`.
    event.target.value = "";
  }

  /**
   * A screenshot pasted straight into the Request field, without a trip to
   * the file picker. `clipboardData.items` carries every kind a paste can
   * hold; only image entries are staged here, and plain text still falls
   * through to the field as text.
   */
  function onRequestPaste(event: ClipboardEvent<HTMLTextAreaElement>): void {
    for (const item of Array.from(event.clipboardData.items)) {
      if (!item.type.startsWith("image/")) continue;
      const file = item.getAsFile();
      if (file !== null) void stage(file);
    }
  }

  return (
    <Card className="armada-dispatch">
      <CardHeader>
        <span className="armada-dispatch__head">
          <CardTitle>Dispatch a job</CardTitle>
          {/* What pressing Dispatch sets off, what approving does, and what a
              proposal freezes. On the title, because it is about the surface
              rather than about any one control on it. */}
          <GuideMark guide={GUIDE_DISPATCH} />
        </span>
        {close}
      </CardHeader>
      <CardContent>
        <div className="armada-dispatch__body">
          <Where
            {...(repository === undefined ? {} : { repository })}
            refs={refs}
            onRefs={onRefs}
            branches={branches}
            disabled={disabled}
          />

          {/* Above the field rather than on the card's head: it swaps what
              is directly under it, and nothing else on the card. */}
          {sketchPad === undefined ? null : (
                <div className="armada-dispatch__modes">
                  <Tabs items={MODES} value={mode} onChange={(id) => onMode?.(id as RequestMode)} />
                </div>
              )}
              {sketchPad !== undefined && mode === "sketch" ? sketchPad : (
              <div className="armada-mention-anchor">
                <Textarea
                  label="Request"
                  rows={4}
                  value={request}
                  placeholder={PLACEHOLDER}
                  disabled={disabled}
                  {...(focused
                    ? { autoFocus: true, onFocus: (event: FocusEvent<HTMLTextAreaElement>) => event.target.setSelectionRange(event.target.value.length, event.target.value.length) }
                    : {})}
                  {...mention.fieldAria}
                  onChange={mention.onFieldChange}
                  onKeyDown={mention.onFieldKeyDown}
                  onSelect={mention.onFieldSelect}
                  onBlur={mention.onFieldBlur}
                  onPaste={onRequestPaste}
                />
                {/* The `@` mention popup, directly under the field it opened
                    on — see `MentionPopover`'s own note on why it is anchored
                    there and not at the caret. */}
                {mention.open ? (
                  <MentionPopover
                    query={mention.query}
                    results={mention.results}
                    active={mention.active}
                    listId={mention.listId}
                    optionId={mention.optionId}
                    onHover={mention.onHover}
                    onChoose={mention.onChoose}
                  />
                ) : null}
              </div>
              )}
              {/* Hidden behind the "Attach" button — no file input is ever
                  drawn directly, the platform's own picker chrome is not this
                  app's to style. */}
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="armada-file-input"
                onChange={onFilesPicked}
              />
              <Attached
                attachments={attachments}
                onRemoveAttachment={onRemoveAttachment}
                links={links}
                {...(onAddLink === undefined ? {} : { onAddLink })}
                {...(onRemoveLink === undefined ? {} : { onRemoveLink })}
                {...(node === undefined ? {} : { node })}
                {...(sketch === undefined ? {} : { sketch })}
                disabled={disabled}
                onPickFile={() => fileInputRef.current?.click()}
              />
              {settings}
              {/* *What happens next* stood here — three numbered lines, true
                  before anything was typed and still true of a job that never
                  ran. #1540 put them on the screen and #1602 took them off:
                  the `?` on the card's own title is where they live now. */}

          {disabledNote === undefined ? null : (
            <p className="armada-dispatch__note">{disabledNote}</p>
          )}
        </div>
      </CardContent>

      <CardFooter className="armada-dispatch__foot">
        {/* One control, because there is one way through this surface. The
            form `Enter by hand` opened is gone: the Settings block took every
            decision it carried, and the owner's call of 2026-09-23 is that
            with those there it is not needed any more.

            **Never pending.** The press leaves this card, so there is no state
            of it in which Fleet has been asked and has not answered. */}
        <Button variant="primary" onClick={onDispatch} disabled={disabled || empty}>
          Dispatch
        </Button>
      </CardFooter>
    </Card>
  );
}

/**
 * The repository, and the two refs.
 *
 * **The repository is a value and the refs are fields.** Which repository is
 * answered before this surface opens; where the work starts and where it lands
 * are this form's, and both are drawn as fields even when they read the same
 * branch — a value only one of them could change would be the pair collapsed
 * back into one.
 *
 * **Both pick over the repository's branches, and only one of them makes
 * one.** Where the work starts has to exist; where it lands may not, and
 * naming a branch that is not there is how one gets made.
 *
 * **The first field reads `Base branch`, and read `From` until 28 Sep 2026.**
 * The owner's own word, and the Manifest's: `base` is the branch worktrees are
 * cut from (`packages/screens/src/draft/landing.ts`), and the list already tags
 * that row `base`, so the field and the row it opens on now agree.
 */
function Where({
  repository,
  refs,
  onRefs,
  branches,
  disabled,
}: {
  repository?: string;
  refs: Refs;
  onRefs: (refs: Refs) => void;
  branches: readonly BranchOption[] | null;
  disabled: boolean;
}) {
  return (
    <div className="armada-dispatch__where">
      {repository === undefined ? null : (
        <div className="armada-dispatch__repository">
          <span className="armada-dispatch__where-label">Repository</span>
          <span className="mono armada-dispatch__where-value">{repository}</span>
        </div>
      )}
      <BranchPicker
        label="Base branch"
        value={refs.from}
        onValue={(from) => onRefs({ ...refs, from })}
        branches={branches}
        disabled={disabled}
      />
      <BranchPicker
        label="Lands in"
        value={refs.target}
        onValue={(target) => onRefs({ ...refs, target })}
        branches={branches}
        offerNew
        disabled={disabled}
      />
    </div>
  );
}

/**
 * What is attached to the request: files, addresses, and the Studio node it
 * came off.
 *
 * **One row of chips and not three lists.** A person reads what they attached,
 * not which mechanism staged it — the kind is a word on the chip.
 */
function Attached({
  attachments,
  onRemoveAttachment,
  links,
  onAddLink,
  onRemoveLink,
  node,
  sketch,
  disabled,
  onPickFile,
}: {
  attachments: readonly StagedAttachment[];
  onRemoveAttachment: (path: string) => void;
  links: readonly string[];
  onAddLink?: (address: string) => void;
  onRemoveLink?: (address: string) => void;
  node?: { name: string };
  sketch?: { name: string };
  disabled: boolean;
  onPickFile: () => void;
}) {
  // The link field is open or it is not. Local, because nothing outside this
  // card reads whether somebody is part-way through pasting an address.
  const [adding, setAdding] = useState(false);
  const [address, setAddress] = useState("");
  const empty = address.trim() === "";

  function add(): void {
    if (empty || onAddLink === undefined) return;
    onAddLink(address.trim());
    setAddress("");
    setAdding(false);
  }

  return (
    <div className="armada-dispatch__attached">
      <div className="armada-dispatch__attach-acts">
        <Button variant="secondary" size="sm" disabled={disabled} onClick={onPickFile}>
          Attach
        </Button>
        {onAddLink === undefined ? null : (
          <Button
            variant="secondary"
            size="sm"
            disabled={disabled}
            onClick={() => setAdding((open) => !open)}
          >
            Add a link
          </Button>
        )}
      </div>
      {!adding ? null : (
        <div className="armada-dispatch__link-field">
          <Input
            label="Link"
            mono
            value={address}
            placeholder="An issue, a pull request, a page"
            disabled={disabled}
            onChange={(event) => setAddress(event.target.value)}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              event.preventDefault();
              add();
            }}
          />
          <Button variant="secondary" size="sm" disabled={disabled || empty} onClick={add}>
            Add
          </Button>
        </div>
      )}
      {attachments.length + links.length === 0 && node === undefined && sketch === undefined ? null : (
        <div className="armada-dispatch__chips">
          {node === undefined ? null : <AttachmentChip filename={node.name} kind="node" />}
          {/* Read-only: a sketch is taken back on the pad, where the boxes
              going are visible, and not by a press on a chip saying only
              `sketch 1`. */}
          {sketch === undefined ? null : <AttachmentChip filename={sketch.name} />}
          {attachments.map((attachment) => (
            <AttachmentChip
              key={attachment.path}
              filename={attachment.filename}
              onRemove={() => onRemoveAttachment(attachment.path)}
            />
          ))}
          {links.map((link) => (
            <AttachmentChip
              key={link}
              filename={link}
              kind="link"
              {...(onRemoveLink === undefined ? {} : { onRemove: () => onRemoveLink(link) })}
            />
          ))}
        </div>
      )}
      {links.length === 0 ? null : (
        <p className="armada-dispatch__said">Links go out with the request, one to a line.</p>
      )}
    </div>
  );
}
