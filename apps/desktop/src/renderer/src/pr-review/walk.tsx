// The Walk tool's body and its two sheets. The body replaces the reader's, as every tool's results do: **how the place was
// found, one line per source tried** (the pull request, what Armada remembers, then him), the walk as a preview (numbered steps
// for a web walk, the requests for an API walk), and the setup a pulled-down change needs once Walk it holds the place up.
// **Walk it opens Armada's own walk window**, the one a Session's `show_window` opens, which plays either. The sheets ask where a
// change runs when nothing says, and list the places Armada remembers (`store-walk.ts`).

import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { ArrowLeft, Check, CircleHelp, Footprints, GitBranch, GitPullRequest, Globe, Hourglass, LoaderCircle, Minus, NotebookText, Pencil, RefreshCw, RotateCw, Send, SquareDashed, SquareTerminal, Trash2, X } from "lucide-react";
import { Button, Checkbox, Input, Kbd, PrMark, Radio, RadioGroup, Sheet, Tooltip, useKept } from "@armada/components";
import type { RememberedPlace, ReviewWalkState, WalkPlace, WalkSource, WalkStep, WalkTarget } from "@armada/protocol";
import { useAtFloor } from "@armada/shell";

import { RunFailed, SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, ToolDef, WalkWork } from "./model";
import { areaShown, placeWords, walkOf } from "./runs";
import type { WalkWritten } from "./runs";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { WALK_RUN } from "./store-walk";
import "./walk.css";

type Said = { Glyph: LucideIcon; said: string; tone?: string };

const SOURCES: Record<WalkSource | "you", Said> = {
  pull_request: { Glyph: GitPullRequest, said: "From the pull request" },
  memory: { Glyph: NotebookText, said: "From what Armada remembers of this area" },
  you: { Glyph: CircleHelp, said: "From you" },
};

const FOUND = { found: { Glyph: Check, said: "Found", tone: "success" }, nothing: { Glyph: Minus, said: "Nothing found" }, asking: { Glyph: CircleHelp, said: "Waiting on you" } } as const satisfies Record<string, Said>;

type Phase = "pending" | "running" | "done" | "failed";

const PHASE: Record<Phase, Said> = {
  pending: { Glyph: SquareDashed, said: "Waiting" },
  running: { Glyph: LoaderCircle, said: "Running", tone: "running" },
  done: { Glyph: Check, said: "Done", tone: "success" },
  failed: { Glyph: X, said: "Failed", tone: "failed" },
};

const SETUP_MARK = { checkout: GitBranch, run: SquareTerminal, wait: Hourglass } as const;

/** A mark that turns while it runs, as the checks mark does. */
function Turning({ running, Glyph, said, tone }: { running: boolean; Glyph: LucideIcon; said: string; tone: string | undefined }) {
  return (
    <span className="pr-review__mark" data-checks={running ? "running" : undefined}>
      <PrMark Glyph={Glyph} said={said} tone={tone} />
    </span>
  );
}

/** A step's verb, its target, and what it types or presses. */
function actOf(step: WalkStep): { verb: string; target: WalkTarget; with?: string } {
  if ("press" in step) return { verb: "press", target: step.press };
  if ("look" in step) return { verb: "look", target: step.look };
  if ("hover" in step) return { verb: "hover", target: step.hover };
  if ("later" in step) return { verb: "look", target: step.later };
  if ("type" in step) return { verb: "type", target: step.into, with: step.type };
  if ("key" in step) return { verb: "key", target: step.on, with: step.key };
  if ("select" in step) return { verb: "select", target: step.in, with: step.select };
  if ("paste" in step) return { verb: "paste", target: step.paste };
  return { verb: "drag", target: step.drag };
}

export function WalkView({ pr, tool }: { pr: ReviewPr; tool: ToolDef }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const walk = work.walk;
  const run = work.runs[tool.id];
  const written = walkOf(work.wire[WALK_RUN]);
  const mark = TOOL_MARK[tool.mark];
  const source = SOURCE_MARK[tool.source];
  const stale = run?.state === "done" && run.head !== pr.head;
  const finding = walk.find.state === "looking";
  const going = run?.state === "running";
  return (
    <div className="pr-results pr-walk" aria-label={`${tool.name} results`} role="region" data-tool={tool.mark} data-source={tool.source}>
      <header className="pr-results__head">
        <Tooltip label="Back to the overview" shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back to the overview" onClick={() => store.setView(pr.key, "explanation")}>
            <ArrowLeft size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <PrMark Glyph={mark.Glyph} said={tool.name} />
        <h3 className="pr-results__title">{tool.name}</h3>
        <PrMark Glyph={source.Glyph} said={source.said} />
        <span className="pr-results__acts">
          <Tooltip label="Walk it again, for this pull request">
            <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={going || finding} onClick={() => store.spend(tool.name, () => store.findPlace(pr.key))}>
              <RotateCw size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {run?.state === "done" && stale ? (
        <p className="pr-results__stale" role="note" aria-label="Ran on an older commit">
          <PrMark Glyph={RefreshCw} said="Out of date" />
          {`Ran on ${run.head.slice(0, 7)}. The pull request is at ${pr.head.slice(0, 7)}.`}
        </p>
      ) : null}
      <Found walk={walk} />
      {walk.said === undefined ? null : <RunFailed said={walk.said} />}
      {walk.asking ? (
        <article className="pr-change pr-walk__ask" tabIndex={0} data-stop aria-label="Question: where it runs">
          <Button variant="primary" size="sm" data-run="" onClick={() => store.askWhere(pr.key)}>
            <CircleHelp size={16} strokeWidth={2} aria-hidden />
            Answer
            <Kbd aria-hidden="true">↵</Kbd>
          </Button>
        </article>
      ) : null}
      {going ? (
        <p className="pr-results__running">
          <Turning running Glyph={LoaderCircle} said="Writing the walk" tone="running" />
        </p>
      ) : null}
      {run?.state === "failed" ? <RunFailed said={run.said} /> : null}
      {run?.state === "done" && written === undefined ? (
        <p className="pr-results__clean">
          <PrMark Glyph={Check} said="Nothing to walk" tone="success" />
        </p>
      ) : null}
      {written === undefined ? null : <Walk pr={pr} work={walk} written={written} />}
      {walk.notes.length === 0 ? null : <Notes notes={walk.notes} />}
    </div>
  );
}

/** One line per source tried, in the order he chose: the pull request, what Armada remembers, then him. */
function Found({ walk }: { walk: WalkWork }) {
  const find = walk.find;
  if (find.state === "idle") return null;
  if (find.state === "looking") {
    return (
      <p className="pr-results__running">
        <Turning running Glyph={LoaderCircle} said="Looking" tone="running" />
      </p>
    );
  }
  if (find.state === "said") return <RunFailed said={find.said} />;
  const { tried, place } = find.found;
  const nothing = tried.every((one) => !one.found);
  const lines: { key: string; source: keyof typeof SOURCES; by: string | undefined; state: keyof typeof FOUND; said: string }[] = tried.map((one) => ({ key: one.source, source: one.source, by: one.by, state: one.found ? "found" : "nothing", said: one.said }));
  if (nothing) lines.push({ key: "you", source: "you", by: undefined, state: place === undefined ? "asking" : "found", said: place === undefined ? "Not in the pull request or in memory" : placeWords(place) });
  return (
    <ol className="pr-walk__facts" aria-label="Where it runs">
      {lines.map((one) => {
        const from = SOURCES[one.source];
        const result = FOUND[one.state];
        return (
          <li key={one.key} className="pr-walk__fact" aria-label={from.said} data-state={one.state}>
            <PrMark Glyph={from.Glyph} said={one.by === undefined ? from.said : `${from.said}: ${one.by}`} />
            <PrMark Glyph={result.Glyph} said={result.said} tone={"tone" in result ? result.tone : undefined} />
            <span className="pr-walk__said">{one.said}</span>
          </li>
        );
      })}
    </ol>
  );
}

/** What a pulled-down change needs before it can be walked, a line at a time as Fleet reports it. */
function Setup({ pr, place, walk }: { pr: ReviewPr; place: WalkPlace; walk: WalkWork }) {
  const state: ReviewWalkState | undefined = walk.held?.state ?? walk.reached;
  if (place.how !== "pull" || state === undefined) return null;
  const failed = walk.held === undefined && walk.reached !== undefined;
  const at = state === "checking_out" ? 0 : state === "starting" ? 1 : 3;
  const phase = (line: number): Phase => (line < at ? "done" : line === at ? (failed ? "failed" : "running") : "pending");
  const lines = [
    { id: "checkout", said: `Check out ${pr.branch} into slot ${walk.held?.slot ?? ""}`.trim(), code: undefined },
    { id: "run", said: "Run", code: place.command },
    { id: "wait", said: "Wait for", code: place.address ?? place.api },
  ] as const;
  return (
    <ol className="pr-walk__facts" aria-label="Setup">
      {lines.map((one, line) => {
        const now = PHASE[phase(line)];
        const Glyph = SETUP_MARK[one.id];
        return (
          <li key={one.id} className="pr-walk__fact" aria-label={`${one.said} ${one.code ?? ""}`.trim()} data-state={phase(line)}>
            <PrMark Glyph={Glyph} said={one.id === "checkout" ? "Checkout" : one.id === "run" ? "Command" : "Address"} />
            <Turning running={phase(line) === "running"} Glyph={now.Glyph} said={now.said} tone={now.tone} />
            <span className="pr-walk__said">
              {one.said}
              {one.code === undefined ? null : <code className="pr-walk__code">{one.code}</code>}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

/** The walk as a preview, and the one act that opens it in Armada's walk window. **There is one way to walk anything**: a web walk and an API walk play in the same window. */
function Walk({ pr, work, written }: { pr: ReviewPr; work: WalkWork; written: WalkWritten }) {
  const store = useReviewStore()!;
  const web = written.shape === "web_walk";
  const place = written.place;
  const base = web ? (place.address ?? place.api ?? "") : (place.api ?? place.address ?? "");
  const empty = web ? written.steps.length === 0 : written.requests.length === 0;
  return (
    <section className="pr-walk__body" aria-label={web ? "Web walk" : "API walk"}>
      <p className="pr-walk__against">
        <PrMark Glyph={web ? Globe : Send} said={web ? "Web address" : "API base address"} />
        <code className="pr-walk__code">{base}</code>
      </p>
      <Setup pr={pr} place={place} walk={work} />
      {empty ? (
        <p className="pr-results__clean">
          <PrMark Glyph={Check} said="Nothing to walk" tone="success" />
        </p>
      ) : (
        <article className="pr-change pr-walk__card" tabIndex={0} data-stop aria-label="Walk it">
          <Button variant="primary" size="sm" data-run="" pending={work.opening} disabled={work.opening} onClick={() => store.walkIt(pr.key)}>
            <Footprints size={16} strokeWidth={2} aria-hidden />
            Walk it
            <Kbd aria-hidden="true">↵</Kbd>
          </Button>
        </article>
      )}
      {written.shape === "web_walk" ? (
        <ol className="pr-steps" aria-label="Steps">
          {written.steps.map((step, at) => {
            const act = actOf(step);
            return (
              <li key={at} className="pr-steps__step" aria-label={`Walk step ${at + 1}`}>
                <span className="pr-steps__n mono">{at + 1}</span>
                <div className="pr-steps__body">
                  <p className="pr-steps__do">{step.say}</p>
                  <p className="pr-walk__act">
                    <code className="pr-walk__verb">{act.verb}</code>
                    <span className="pr-walk__target">{act.target.name === undefined ? act.target.role : `${act.target.role} “${act.target.name}”`}</span>
                    {act.with === undefined ? null : <code className="pr-walk__code">{act.with}</code>}
                  </p>
                </div>
              </li>
            );
          })}
        </ol>
      ) : (
        <ol className="pr-steps" aria-label="Requests">
          {written.requests.map((step, at) => (
            <li key={at} className="pr-steps__step" aria-label={`Request ${at + 1}: ${step.method} ${step.path}`}>
              <span className="pr-steps__n mono">{at + 1}</span>
              <div className="pr-steps__body">
                <p className="pr-steps__do">{step.say}</p>
                <p className="pr-walk__act">
                  <code className="pr-walk__verb">{step.method}</code>
                  <code className="pr-walk__path">{step.path}</code>
                </p>
                {step.body === undefined ? null : (
                  <pre className="pr-walk__json" aria-label="Request body">
                    {step.body}
                  </pre>
                )}
                <p className="pr-steps__look">{step.look}</p>
              </div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

/** Notes taken in the walk window where no Session holds the pull request. They stay in Armada and are never posted. */
function Notes({ notes }: { notes: WalkWork["notes"] }) {
  return (
    <ul className="pr-walk__notes" aria-label="Notes">
      {notes.map((one, at) => (
        <li key={`${one.at}:${at}`} className="pr-walk__note">
          <PrMark Glyph={NotebookText} said="Armada's note, not posted" />
          <span className="pr-walk__said">{one.text}</span>
          <code className="pr-walk__code">{one.location}</code>
        </li>
      ))}
    </ul>
  );
}

// ---- The sheets ---------------------------------------------------------------------------------------------

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** Both sheets. Mounted with the Flight control, which is there wherever the review is. */
export function WalkSheets() {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const floor = useAtFloor();
  const { sheet, prs } = reviews;
  const pr = sheet?.kind === "ask" ? prs.find((one) => one.key === sheet.key) : undefined;
  const find = pr === undefined ? undefined : reviews.work[pr.key]?.walk.find;
  return (
    <>
      <Sheet kind="walk-place" open={pr !== undefined && find?.state === "found"} floating floor={floor} title="Where it runs" closeLabel="Close" closeBinding="Esc" onClose={() => store.closeSheet()}>
        {pr === undefined || find?.state !== "found" ? null : <AskPlace pr={pr} area={find.found.area} />}
      </Sheet>
      <Sheet kind="walk-places" open={sheet?.kind === "places"} floating floor={floor} title="Where changes run" closeLabel="Close" closeBinding="Esc" onClose={() => store.closeSheet()}>
        <Places />
      </Sheet>
    </>
  );
}

/** Asks where this change runs, and offers to remember it for the area of code it touches. */
function AskPlace({ pr, area: first }: { pr: ReviewPr; area: string }) {
  const store = useReviewStore()!;
  const id = `pr-review:walk-place:${pr.key}`;
  const [how, setHow, forgetHow] = useKept<WalkPlace["how"]>(`${id}:how`, "deployed");
  const [address, setAddress, forgetAddress] = useKept(`${id}:address`, "");
  const [command, setCommand, forgetCommand] = useKept(`${id}:command`, "");
  const [api, setApi, forgetApi] = useKept(`${id}:api`, "");
  const [area, setArea] = useState(first);
  const [remember, setRemember] = useState(true);
  const where = address.trim() !== "" || api.trim() !== "";
  const ready = where && (how === "deployed" || command.trim() !== "");
  useEffect(() => {
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>(".pr-walk-ask input[type='radio']:checked")?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, []);
  const save = () => {
    if (!ready) return;
    const place: WalkPlace = { how, ...(address.trim() === "" ? {} : { address: address.trim() }), ...(how === "pull" ? { command: command.trim() } : {}), ...(api.trim() === "" ? {} : { api: api.trim() }) };
    store.answer(pr.key, place, remember ? area.trim().replace(/\/$/, "") : undefined);
    [forgetHow, forgetAddress, forgetCommand, forgetApi].forEach((forget) => forget());
  };
  return (
    <form
      className="pr-add pr-walk-ask"
      aria-label="Where it runs"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
      onKeyDown={(event) => {
        // Enter sends the form from a choice as it does from a box, and from the box without waiting on the browser to do it.
        if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
          event.preventDefault();
          save();
        }
      }}
    >
      <RadioGroup label="Where">
        <span className="pr-add__choice">
          <Radio name="how" checked={how === "deployed"} onChange={() => setHow("deployed")}>
            Deployed at
          </Radio>
        </span>
        <span className="pr-add__choice">
          <Radio name="how" checked={how === "pull"} onChange={() => setHow("pull")}>
            Pull it down and run
          </Radio>
        </span>
      </RadioGroup>
      {how === "pull" ? <Input mono label="Command" value={command} onChange={(event) => setCommand(event.target.value)} /> : null}
      <Input mono label="Address" value={address} onChange={(event) => setAddress(event.target.value)} />
      <Input mono label="API base address" value={api} onChange={(event) => setApi(event.target.value)} />
      <div className="pr-walk-ask__remember">
        <Checkbox checked={remember} onChange={(event) => setRemember(event.target.checked)}>
          Remember for
        </Checkbox>
        <Input mono aria-label="Area" value={areaShown(area)} disabled={!remember} onChange={(event) => setArea(event.target.value)} />
      </div>
      <div className="pr-add__bar">
        <Button variant="ghost" onClick={() => store.closeSheet()}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={!ready}>
          Walk
        </Button>
      </div>
    </form>
  );
}

/** The places Armada remembers, by repository and area, each one editable or forgotten. */
function Places() {
  const { places } = useReviews();
  const [editing, setEditing] = useState<string | undefined>(undefined);
  return (
    <>
      {places.said === undefined ? null : <RunFailed said={places.said} />}
      {places.list.length === 0 ? null : (
        <ul
          className="pr-walk-places"
          aria-label="Remembered places"
          onKeyDown={(event) => {
            // Down and up, and j and k, move between the rows; the row's own keys are on the row.
            if (typing(event.target)) return;
            const rows = [...event.currentTarget.querySelectorAll<HTMLElement>("article[data-stop]")];
            const at = rows.findIndex((one) => one === event.target || one.contains(event.target as Node));
            const by = event.key === "ArrowDown" || event.key === "j" ? 1 : event.key === "ArrowUp" || event.key === "k" ? -1 : 0;
            if (by !== 0) {
              event.preventDefault();
              rows[Math.min(rows.length - 1, Math.max(0, at + by))]?.focus();
            }
          }}
        >
          {places.list.map((one) => (
            <li key={one.id}>
              <Row one={one} editing={editing === one.id} onEdit={(on) => setEditing(on ? one.id : undefined)} />
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

function Row({ one, editing, onEdit }: { one: RememberedPlace; editing: boolean; onEdit: (on: boolean) => void }) {
  const store = useReviewStore()!;
  const [command, setCommand] = useState(one.place.command ?? "");
  const [address, setAddress] = useState(one.place.address ?? "");
  const [api, setApi] = useState(one.place.api ?? "");
  const reset = () => {
    setCommand(one.place.command ?? "");
    setAddress(one.place.address ?? "");
    setApi(one.place.api ?? "");
  };
  const ready = address.trim() !== "" || api.trim() !== "";
  return (
    <article
      className="pr-change pr-walk-place"
      tabIndex={0}
      data-stop
      aria-label={`Remembered: ${areaShown(one.area)} in ${one.repository}`}
      onKeyDown={(event) => {
        if (typing(event.target) || event.target !== event.currentTarget) return;
        if (event.key === "e" || event.key === "o" || event.key === "Enter") {
          event.preventDefault();
          reset();
          onEdit(true);
        } else if (event.key === "Delete" || event.key === "Backspace") {
          event.preventDefault();
          store.forgetPlace(one);
        }
      }}
    >
      <header className="pr-change__head">
        <code className="pr-walk__path">{areaShown(one.area)}</code>
        <span className="pr-change__path">{one.repository}</span>
        <span className="pr-change__marks">
          <Tooltip label="Edit" shortcut="e">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Edit ${areaShown(one.area)}`} onClick={() => (reset(), onEdit(!editing))}>
              <Pencil size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          <Tooltip label="Forget" shortcut="Del">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Forget ${areaShown(one.area)}`} onClick={() => store.forgetPlace(one)}>
              <Trash2 size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {editing ? (
        <form
          className="pr-add"
          aria-label={`Edit ${areaShown(one.area)}`}
          onSubmit={(event) => {
            event.preventDefault();
            if (!ready) return;
            store.editPlace(one, { how: one.place.how, ...(address.trim() === "" ? {} : { address: address.trim() }), ...(one.place.how === "pull" ? { command: command.trim() } : {}), ...(api.trim() === "" ? {} : { api: api.trim() }) });
            onEdit(false);
          }}
        >
          {one.place.how === "pull" ? <Input mono label="Command" value={command} onChange={(event) => setCommand(event.target.value)} /> : null}
          <Input mono label="Address" value={address} onChange={(event) => setAddress(event.target.value)} />
          <Input mono label="API base address" value={api} onChange={(event) => setApi(event.target.value)} />
          <div className="pr-add__bar">
            <Button variant="ghost" onClick={() => onEdit(false)}>
              Cancel
            </Button>
            <Button variant="primary" type="submit" disabled={!ready}>
              Save
            </Button>
          </div>
        </form>
      ) : (
        <p className="pr-change__why">{placeWords(one.place)}</p>
      )}
    </article>
  );
}
