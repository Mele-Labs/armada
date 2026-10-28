// One agent's act on its Job browser, done and answered — `docs/practices/protocol/the-job-browser.md`. Fleet holds the
// agent's call until Bridge posts `answer_browser_request`, so **every request is answered once**, with an answer or an
// error, whatever happens on the way. Targets are found as a walk's steps are (`capture/finder.ts`) and acted on with real
// input (`capture/stepwalk.ts`), so the page hears a pointer and a keyboard.

import type { AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserError, BrowserRequest, WalkTarget } from "@armada/protocol";

import type { CaptureRect } from "../shared/capture-window";
import type { HeldKind } from "./capture/finder";
import { PATIENCE_MS, pressEvents, typeEvents, type Sent } from "./capture/stepwalk";

/** What an act does to one Job's page: Electron's view in the app (`job-browsers.ts`), a fake in a test. */
export type JobPage = {
  url(): string;
  title(): string;
  /** Load an address and wait for it: `loaded`, `failed` with Chromium's words, or `timed_out`. */
  load(url: string, patience: number): Promise<{ loaded: true } | { failed: string } | { timedOut: true }>;
  /** Find the target, scroll it into view, and hold it for `focus`. `null`: not on the page within `patience`. */
  find(target: WalkTarget, patience: number): Promise<CaptureRect | null>;
  focus(): Promise<HeldKind | null>;
  send(sent: Sent): void;
  selectAll(): void;
  /** A beat, so the page has drawn what the input did before it is read. */
  settle(): Promise<void>;
  snapshot(): Promise<string>;
  /** The page as a PNG at `path`. False: nothing could be taken or written. */
  capture(path: string): Promise<boolean>;
  /** `expression` in a world of the page's own, its value as `JSON.stringify` writes it. */
  evaluate(expression: string): Promise<{ result?: string } | { threw: string }>;
};

export type ActsHost = {
  page(sessionId: string): JobPage | null;
  ownerDriving(sessionId: string): boolean;
  /**
   * **An agent's `open` on a Job with no Job browser opens one in that Job's side panel**, on the address. The owner has not
   * decided this yet: where he says the agent may not show a page unasked, this one branch refuses instead.
   */
  openClosedInPanel(sessionId: string, url: string): JobPage;
  answer(body: AnswerBrowserRequest): Promise<unknown>;
};

/** An address finishes loading within this, or the agent is told it did not: Fleet holds the call 20 seconds, and the snapshot after it takes up to 7. */
export const LOAD_MS = 12_000;

export const CLOSED = "The Job browser is closed. Call browser_open first.";

type Reply = { answer: BrowserAnswer } | { error: BrowserError };

const failed = (why: string): Reply => ({ error: { kind: "failed", why } });

async function pageOf(page: JobPage): Promise<Reply> {
  return { answer: { kind: "page", url: page.url(), title: page.title(), snapshot: await page.snapshot() } };
}

async function loaded(page: JobPage, url: string): Promise<Reply> {
  if (!/^https?:\/\//i.test(url)) return failed(`${url} is not an http or https address.`);
  const load = await page.load(url, LOAD_MS);
  if ("timedOut" in load) return { error: { kind: "timed_out" } };
  if ("failed" in load) return failed(`${url} did not load: ${load.failed}`);
  return pageOf(page);
}

async function done(host: ActsHost, sessionId: string, act: BrowserAct): Promise<Reply> {
  const page = host.page(sessionId);
  if (page === null) return act.kind === "open" ? loaded(host.openClosedInPanel(sessionId, act.url), act.url) : failed(CLOSED);
  if (host.ownerDriving(sessionId)) return { error: { kind: "owner_driving" } };
  switch (act.kind) {
    case "open":
      return loaded(page, act.url);
    case "click": {
      const box = await page.find(act.target, PATIENCE_MS);
      if (box === null) return { error: { kind: "not_found" } };
      for (const event of pressEvents(box)) page.send(event);
      await page.settle();
      return pageOf(page);
    }
    case "type": {
      if ((await page.find(act.target, PATIENCE_MS)) === null) return { error: { kind: "not_found" } };
      // Typed over, not into, as a walk's `type` is.
      if ((await page.focus()) === "field") page.selectAll();
      for (const event of typeEvents(act.submit === true ? `${act.text}\n` : act.text)) page.send(event);
      await page.settle();
      return pageOf(page);
    }
    case "snapshot":
      return pageOf(page);
    case "screenshot":
      return (await page.capture(act.path)) ? { answer: { kind: "screenshot", url: page.url(), title: page.title() } } : failed("No picture of the page could be taken.");
    case "wait_for": {
      const target = act.target ?? (act.text === undefined ? undefined : { role: "text", name: act.text });
      if (target === undefined) return failed("wait_for names neither text nor a target.");
      return (await page.find(target, act.timeout_ms)) === null ? { error: { kind: "timed_out" } } : pageOf(page);
    }
    case "evaluate": {
      const ran = await page.evaluate(act.expression);
      if ("threw" in ran) return failed(ran.threw);
      return { answer: { kind: "evaluated", url: page.url(), title: page.title(), ...(ran.result === undefined ? {} : { result: ran.result }) } };
    }
  }
}

/** Do the act and answer it. A throw on the way is answered as `failed`, so the agent's call never waits out Fleet's hold. */
export async function actOn(host: ActsHost, request: BrowserRequest): Promise<void> {
  let reply: Reply;
  try {
    reply = await done(host, request.session_id, request.act);
  } catch (error) {
    reply = failed(error instanceof Error ? error.message : String(error));
  }
  await host.answer({ request_id: request.request_id, ...reply });
}
