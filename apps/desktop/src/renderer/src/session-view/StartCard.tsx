// The start card: what a Job dispatched held settles before any agent moves. A header band over a mono body, labels in one column and
// controls in the next, a mark column after them. Title, Message (the first message the agent will read, where Fleet gives it), Kind, Lands
// (hidden when the kind has no landing step), Model and Effort. Start begins the agent, the card goes, and the Job view carries on.
//
// **Keyboard.** Focus lands on the card. Up and Down move between rows, Left and Right change a row's choice, Tab goes through the fields,
// Enter starts from anywhere (a title being edited is kept first), and Escape in the title or the message puts the edit back. In the message
// Enter is a newline, Up and Down move the caret and leave the box only from its first or last position, and ⌘↵ starts. The title is kept as a
// draft until it is committed on leaving the field or on Start; the message is kept until Start. The marks are read by hovering them and are
// not tab stops.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Lightbulb, Quote, Repeat, TextCursor } from "lucide-react";
import { Button, Input, Kbd, Radio, Select, Textarea, Tooltip, useKept } from "@armada/components";
import type { DeliveryChoice, RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import "./start-card.css";

/** How a Job's work lands, in the order Fleet offers it: `DeliveryChoice::label` and `detail` in `crates/ipc/src/session_delivery.rs`. */
const LANDS: readonly { choice: DeliveryChoice; label: string; detail: string }[] = [
  { choice: "auto_land", label: "Auto land", detail: "Open a pull request, watch ci, merge it." },
  { choice: "open_pr", label: "Open PR only", detail: "Stop at the open pull request." },
  { choice: "keep_local", label: "Keep local", detail: "Stop at the commit on the job's branch. Nothing is pushed." },
];

const plural = (name: string): string => {
  const lower = name.toLowerCase();
  return /(s|x|ch|sh)$/.test(lower) ? `${lower}es` : `${lower}s`;
};

/** The control a row's arrows and focus go to: the checked radio of a set, or the field. */
const controlOf = (row: HTMLElement): HTMLElement | null => row.querySelector<HTMLElement>("input[type=radio]:checked") ?? row.querySelector<HTMLElement>("input, select, textarea");

const enabled = (control: HTMLElement | null): control is HTMLElement => control !== null && !(control as HTMLInputElement).disabled;

export function StartCard({ session, start, repositories }: { session: Session; start: NonNullable<Session["start"]>; repositories: readonly RepositorySummary[] }) {
  const draft = useSessionsDraft();
  const card = useRef<HTMLElement>(null);
  const { id } = session;
  const proposal = session.work?.proposal;
  const reading = proposal?.reading === true;
  const kind = proposal?.workflow ?? null;
  const picked = proposal?.picked ?? [];
  const kinds = (draft?.work?.workflows ?? []).filter((one) => one.manifestId === undefined || session.manifestId === undefined || one.manifestId === session.manifestId);
  const kindName = kinds.find((one) => one.id === kind)?.name;
  const repository = repositories.find((one) => one.manifest?.id === session.manifestId)?.manifest?.repository;
  const [chosen, setChosen] = useState<DeliveryChoice | undefined>(undefined);
  const [typed, setTyped, forgetTyped] = useKept<string | undefined>(`start:${id}:title`, undefined);
  const [edited, setEdited, forgetEdited] = useKept<string | undefined>(`start:${id}:message`, undefined);
  const title = session.title ?? "";
  // The message row is drawn where Fleet gave the first message; what is sent is his edit, else that.
  const said = start.message === undefined ? undefined : (edited ?? start.message);
  const lands = start.asksDelivery;
  const landing = chosen ?? start.picked?.choice;
  const ready = !reading && draft?.begin !== undefined && (!lands || landing !== undefined) && (said === undefined || said.trim() !== "");

  // Focus lands on the card when it opens.
  useEffect(() => card.current?.focus({ preventScroll: true }), []);

  // The message box grows with its text; the stylesheet caps it and it scrolls beyond the cap.
  useLayoutEffect(() => {
    const box = card.current?.querySelector<HTMLTextAreaElement>("textarea");
    if (box == null) return;
    box.style.height = "auto";
    box.style.height = `${box.scrollHeight + box.offsetHeight - box.clientHeight}px`;
  }, [said]);

  /** The title as he left it goes to Fleet; a blank or unchanged one is let go. */
  function commit() {
    const next = typed?.trim();
    forgetTyped();
    if (next !== undefined && next !== "" && next !== title) draft?.rename?.(id, next);
  }

  function begin() {
    if (!ready || draft?.begin === undefined) return;
    commit();
    const message = said;
    forgetEdited();
    void draft.begin(id, lands ? landing : undefined, message);
  }

  function tune(next: { model: string | null; effort: string | null }) {
    if (draft === undefined) return;
    draft.tune(id, { ...next, mode: session.mode ?? "auto" });
    // A value the proposer picked loses its mark once he changes it.
    if (next.model !== (session.model ?? null)) draft.work?.unpick(id, "model");
    if (next.effort !== (session.effort ?? null)) draft.work?.unpick(id, "effort");
  }

  const chooseKind = (next: string) => draft?.work?.choose(id, next === "" ? null : next);

  const rows = () => Array.from(card.current?.querySelectorAll<HTMLElement>("[data-row]") ?? []);

  /** Up and Down: the next row that has a control to be on. From the card itself, Down is the first and Up the last. */
  function move(by: 1 | -1) {
    const all = rows();
    const at = all.findIndex((one) => one.contains(document.activeElement));
    for (let next = at === -1 ? (by === 1 ? 0 : all.length - 1) : at + by; next >= 0 && next < all.length; next += by) {
      const control = controlOf(all[next]!);
      if (enabled(control)) return control.focus();
    }
  }

  /** Left and Right: the row's choice a step along, without wrapping. */
  function step(row: string, by: 1 | -1) {
    const along = <T,>(items: readonly T[], at: number): T => items[Math.max(0, Math.min(items.length - 1, at + by))]!;
    if (row === "kind") {
      const ids = kinds.map((one) => one.id);
      chooseKind(along(ids, ids.indexOf(kind ?? "")));
    } else if (row === "lands") {
      const next = along(LANDS, LANDS.findIndex((one) => one.choice === landing)).choice;
      setChosen(next);
      card.current?.querySelector<HTMLElement>(`input[type=radio][value=${next}]`)?.focus();
    } else if (row === "model") {
      const models = ["", ...(draft?.models ?? [])];
      tune({ model: along(models, models.indexOf(session.model ?? "")) || null, effort: session.effort ?? null });
    } else if (row === "effort") {
      const efforts = ["", ...(draft?.efforts ?? [])];
      tune({ model: session.model ?? null, effort: along(efforts, efforts.indexOf(session.effort ?? "")) || null });
    }
  }

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.defaultPrevented || event.nativeEvent.isComposing) return;
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>("[data-row]")?.dataset.row;
    const plain = !event.shiftKey && !event.altKey && !event.metaKey && !event.ctrlKey;
    const inMessage = target instanceof HTMLTextAreaElement;
    if (event.key === "Enter" && inMessage && (event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      if (ready) begin();
    } else if (event.key === "Enter" && plain) {
      // Start's own press is a click, which begins; in the message Enter is a newline.
      if (target instanceof HTMLButtonElement || inMessage) return;
      event.preventDefault();
      if (ready) begin();
      // Nothing preselected: the row he has to answer.
      else if (!reading && lands && landing === undefined) controlOf(rows().find((one) => one.dataset.row === "lands")!)?.focus();
    } else if (event.key === "Escape" && ((row === "title" && typed !== undefined) || (row === "message" && edited !== undefined))) {
      event.preventDefault();
      event.stopPropagation();
      event.nativeEvent.stopImmediatePropagation();
      if (row === "title") forgetTyped();
      else forgetEdited();
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && plain) {
      // In the message the caret moves first: the box is left only from its start going up, or its end going down.
      if (inMessage) {
        const { selectionStart, selectionEnd, value } = target;
        if (event.key === "ArrowUp" ? selectionStart !== 0 || selectionEnd !== 0 : selectionStart !== value.length || selectionEnd !== value.length) return;
      }
      event.preventDefault();
      move(event.key === "ArrowDown" ? 1 : -1);
    } else if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && plain && row !== undefined && row !== "title" && row !== "message") {
      event.preventDefault();
      step(row, event.key === "ArrowRight" ? 1 : -1);
    }
  }

  const bulb = (value: "workflow" | "model" | "effort") =>
    picked.includes(value) ? (
      <Tooltip label="Picked by the proposer" decorative>
        <span className="sv-start__mark" role="img" aria-label="Picked by the proposer">
          <Lightbulb size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    ) : null;

  // The standing choice is marked while it is the one in use: his usual, or the one his words named.
  const standing = start.picked !== undefined && landing === start.picked.choice ? start.picked.from : undefined;
  const standingSaid =
    standing === "named" ? "Named in your request" : kindName === undefined || repository === undefined ? "Your usual" : `Your usual for ${plural(kindName)} in ${repository}`;

  return (
    <section ref={card} className="sv-start" aria-label="Start" tabIndex={-1} onKeyDown={onKeyDown}>
      <header className="sv-start__band">
        <h3 className="sv-start__label">{repository ?? ""}</h3>
        <span className="sv-start__acts">
          <Kbd>↵</Kbd>
          <Button variant="primary" size="sm" disabled={!ready} onClick={begin}>
            Start
          </Button>
        </span>
      </header>
      <div className="sv-start__body">
        <div className="sv-start__row" data-row="title">
          <span className="sv-start__name" aria-hidden>
            Title
          </span>
          {reading ? (
            <Tooltip label="Reading the request" decorative>
              <span className="sv-start__reading" role="img" aria-label="Reading the request">
                <TextCursor size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : (
            <Input aria-label="Title" value={typed ?? title} onChange={(event) => setTyped(event.target.value)} onBlur={commit} />
          )}
          <span />
        </div>
        {said === undefined ? null : (
          <div className="sv-start__row" data-row="message">
            <span className="sv-start__name" aria-hidden>
              Message
            </span>
            <Textarea aria-label="Message" rows={3} spellCheck={false} value={said} onChange={(event) => setEdited(event.target.value)} />
            <span />
          </div>
        )}
        <div className="sv-start__row" data-row="kind">
          <span className="sv-start__name" aria-hidden>
            Kind
          </span>
          <Select aria-label="Kind" value={kind ?? ""} disabled={reading} onChange={(event) => chooseKind(event.target.value)}>
            {kind === null ? <option value="" /> : null}
            {kinds.map((one) => (
              <option key={one.id} value={one.id}>
                {one.name}
              </option>
            ))}
          </Select>
          {bulb("workflow")}
        </div>
        {lands ? (
          <div className="sv-start__row" data-row="lands">
            <span className="sv-start__name" aria-hidden>
              Lands
            </span>
            <div className="sv-start__lands" role="radiogroup" aria-label="Lands">
              {LANDS.map((one) => (
                <Tooltip key={one.choice} label={one.detail}>
                  <span>
                    <Radio name={`lands:${id}`} value={one.choice} checked={landing === one.choice} disabled={reading} onChange={() => setChosen(one.choice)}>
                      {one.label}
                    </Radio>
                  </span>
                </Tooltip>
              ))}
            </div>
            {standing === undefined ? (
              <span />
            ) : (
              <Tooltip label={standingSaid} decorative>
                <span className="sv-start__mark" role="img" aria-label={standingSaid}>
                  {standing === "named" ? <Quote size={12} strokeWidth={2} aria-hidden /> : <Repeat size={12} strokeWidth={2} aria-hidden />}
                </span>
              </Tooltip>
            )}
          </div>
        ) : null}
        <div className="sv-start__row" data-row="model">
          <span className="sv-start__name" aria-hidden>
            Model
          </span>
          <Select aria-label="Model" value={session.model ?? ""} disabled={reading} onChange={(event) => tune({ model: event.target.value === "" ? null : event.target.value, effort: session.effort ?? null })}>
            <option value="">{draft?.defaultModel ?? "Auto"}</option>
            {(draft?.models ?? []).map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
          {bulb("model")}
        </div>
        <div className="sv-start__row" data-row="effort">
          <span className="sv-start__name" aria-hidden>
            Effort
          </span>
          <Select aria-label="Effort" value={session.effort ?? ""} disabled={reading} onChange={(event) => tune({ model: session.model ?? null, effort: event.target.value === "" ? null : event.target.value })}>
            <option value="">Auto</option>
            {(draft?.efforts ?? []).map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
          {bulb("effort")}
        </div>
      </div>
    </section>
  );
}
