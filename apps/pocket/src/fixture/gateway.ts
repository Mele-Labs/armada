// A fixture Phone Gateway: answers the phone's routes for one Mac from `wire.ts`. Shared by the fixture
// server (`vite.config.ts`, mode `fixture`) and the acceptance test, so both read the same Macs.
// It checks that a call is signed, not that the signature is good: there is no pairing behind it.

import type {
  AnswerItems,
  AnswerPart,
  AnswerReadyToTry,
  AnswerWaiting,
  JobArtifact,
  ChangeWaiting,
  DismissWaiting,
  SessionRecord,
  SessionRow,
  SessionThread,
  TuneSession,
} from "../wire";

import type { OperatorBody, PhoneSession, QueuedAct } from "../gateway";
import { DESIGN_PAGE, sealedPage, SHOTS } from "./artifacts";
import { decide } from "./operator";
import { toReviewOf } from "./review";
import type { FixtureMac } from "./wire";

export type Answered = {
  status: number;
  type?: string;
  body: string;
  /** A file under `fixture/media/` the server sends in place of `body`. */
  file?: string;
  /** Held this long first, so a phone's skeleton can be seen. */
  delay?: number;
  /** A message the Mac's `/api/live` sends this long after the answer. */
  then?: {
    after: number;
    change: unknown;
  };
};

const MEDIA_DELAY = 800;
const MEDIA_ADDRESS = "/fixture-media/";
const typeOfMedia = (file: string) => (file.endsWith(".mp4") ? "video/mp4" : "image/jpeg");

/** What the fixture was sent, oldest first, for a test to read. */
export type Posted = {
  path: string;
  body: unknown;
};

const json = (value: unknown, status = 200): Answered => ({
  status,
  type: "application/json",
  body: JSON.stringify(value),
});
const said = (status: number, sentence: string): Answered => ({
  status,
  type: "text/plain",
  body: sentence,
});

export function answer(
  mac: FixtureMac,
  posted: Posted[],
  method: string,
  target: string,
  body: string,
  signed: boolean,
): Answered {
  const url = new URL(target, "http://fixture");
  const path = url.pathname;
  if (method === "POST" && path === "/pair")
    return json(
      {
        device_id: `fixture-${mac.name}`,
      },
      202,
    );
  // Unsigned, as the Gateway's own video address is: a video element cannot sign.
  if (path.startsWith(MEDIA_ADDRESS)) {
    const file = path.slice(MEDIA_ADDRESS.length);
    return {
      status: 200,
      type: typeOfMedia(file),
      body: "",
      file,
    };
  }
  if (!path.startsWith("/api/")) return said(404, "Not found.");
  if (!signed) return said(400, "This request is not signed.");
  const id = url.searchParams.get("session_id") ?? "";
  if (method === "GET") {
    switch (path) {
      case "/api/machine":
        return json({
          name: mac.name,
        });
      case "/api/needs":
        return json({
          sessions: mac.needs,
        });
      case "/api/sessions":
        return json({
          sessions: mac.live,
        });
      case "/api/repositories":
        return json(mac.repositories);
      case "/api/queued":
        return json({
          queued: mac.queued ?? [],
        });
      case "/api/plan_usage":
        return json(mac.usage);
      case "/api/models":
        return json(mac.models);
      case "/api/sessions/one": {
        const thread = mac.threads[id];
        if (thread === undefined) return said(404, "No Job has that id on this Mac.");
        // The Gateway's window (`crates/pocket/src/thread_window.rs`): the newest `last` rows before `before`.
        const last = url.searchParams.get("last");
        if (last === null) return json(thread);
        const before = url.searchParams.get("before");
        const at = before === null ? -1 : thread.rows.findIndex((row) => row.id === before);
        const end = at < 0 ? thread.rows.length : at;
        const start = Math.max(0, end - Number(last));
        return json({
          session: thread.session,
          rows: thread.rows.slice(start, end),
          ...(start > 0
            ? {
                older: true,
              }
            : {}),
        });
      }
      case "/api/sessions/kept_styles": {
        const styles = mac.kept[url.searchParams.get("hash") ?? ""];
        return styles === undefined
          ? said(404, "Fleet holds no kept styles under that hash.")
          : json(styles);
      }
      case "/api/sessions/shown/address": {
        const media = mac.media?.[id]?.[url.searchParams.get("path") ?? ""];
        if (media === undefined) return said(404, "That Job did not show that file.");
        return {
          ...json({
            address: `${MEDIA_ADDRESS}${media.file}`,
            media_type: typeOfMedia(media.file),
          }),
          delay: MEDIA_DELAY,
        };
      }
      case "/api/sessions/shown": {
        const media = mac.media?.[id]?.[url.searchParams.get("path") ?? ""];
        if (media !== undefined)
          return {
            status: 200,
            type: typeOfMedia(media.file),
            body: "",
            file: media.file,
            delay: MEDIA_DELAY,
          };
        const text = mac.shown[id]?.[url.searchParams.get("path") ?? ""];
        return text === undefined
          ? said(404, "That Job did not show that file.")
          : json({
              path: url.searchParams.get("path"),
              text,
            });
      }
      case "/api/sessions/to_review":
        return json(toReviewOf(mac));
      case "/api/sessions/artifacts":
        return mac.threads[id] === undefined
          ? said(404, "No Job has that id on this Mac.")
          : json(mac.artifacts?.[id] ?? []);
      case "/api/sessions/artifacts/page": {
        const revision = Number(url.searchParams.get("revision") ?? "2");
        // The design page, and any page a Job's artifacts hold, drawn as the design page is.
        const held = (mac.artifacts?.[id] ?? []).some(
          (one) => one.id === url.searchParams.get("artifact") && one.form === "page",
        );
        if (
          (url.searchParams.get("artifact") !== DESIGN_PAGE && !held) ||
          revision < 1 ||
          revision > 2
        )
          return json(
            {
              code: "fleet.artifact_not_held",
              message: "that Job holds no such page",
            },
            422,
          );
        return {
          status: 200,
          type: "text/html",
          body: sealedPage(revision),
        };
      }
      case "/api/sessions/thread":
        return mac.threads[id] === undefined
          ? said(404, "No Job has that id on this Mac.")
          : json({
              session_id: id,
              head: {},
              items: (mac.tries?.[id] ?? []).map((card) => ({
                kind: "ready_to_try",
                ...card,
              })),
            });
      case "/api/sessions/repository_file": {
        const shot = SHOTS[url.searchParams.get("path") ?? ""];
        return shot === undefined
          ? json(
              {
                code: "fleet.repository_file_not_addressable",
                message: "that file is not one the Job named",
              },
              422,
            )
          : json(shot);
      }
      case "/api/push/key":
        return said(404, "Push is not served by the fixture.");
    }
    return said(404, "Not found.");
  }
  if (method === "POST") {
    const sent = body === "" ? undefined : (JSON.parse(body) as Record<string, unknown>);
    posted.push({
      path,
      body: sent,
    });
    if (path === "/api/sessions/dispatch") return dispatched(mac, sent as unknown as OperatorBody);
    if (path === "/api/sessions/items/answer")
      return itemsAnswered(mac, sent as unknown as AnswerItems);
    if (path === "/api/queued/take_back") return tookBack(mac, String(sent?.["id"] ?? ""));
    if (path === "/api/sessions")
      return json(
        {
          session: mac.live[0],
        },
        201,
      );
    return (
      act(mac, path, sent ?? {}) ?? {
        status: 204,
        body: "",
      }
    );
  }
  return said(405, "Not allowed.");
}

/** The Operator's way with a send: a Mac asleep keeps it (202), any other holds it and says what it did a moment later. */
let sentOn = 0;
function dispatched(mac: FixtureMac, send: OperatorBody): Answered {
  sentOn += 1;
  const on = send.mac ?? mac.name;
  if (send.mac === undefined && !mac.repositories.includes(send.repository))
    return said(400, "Armada does not know that repository. Pick one from the list.");
  if (send.mac !== undefined && mac.unshared)
    return said(
      409,
      "That repository is not in a project shared between your Macs, so a Job cannot start there from here.",
    );
  if (send.mac !== undefined && (mac.asleep ?? []).includes(send.mac)) {
    const queued: QueuedAct = {
      id: `q${sentOn}`,
      mac: send.mac,
      act: {
        kind: "dispatch",
        body: {
          request: send.text,
          ...(send.kind === undefined
            ? {}
            : {
                kind: send.kind,
              }),
          then: [
            {
              message: {
                text: send.text,
              },
            },
          ],
        },
      },
      since: Date.now(),
    };
    (mac.queued ??= []).push(queued);
    return json(queued, 202);
  }
  const id = `d${sentOn}`;
  return {
    ...json({
      operator: id,
    }),
    then: {
      after: 700,
      change: {
        change: "operator",
        dispatched: decide(on, id, send),
      },
    },
  };
}

/** `POST /api/queued/take_back`, as Fleet answers it: the list that is left, or 404 for an id nothing waits under. */
function tookBack(mac: FixtureMac, id: string): Answered {
  const queued = mac.queued ?? [];
  if (!queued.some((one) => one.id === id))
    return json(
      {
        code: "fleet.queued_not_found",
        message: "Nothing waits under that id. It may have been sent already.",
      },
      404,
    );
  mac.queued = queued.filter((one) => one.id !== id);
  return json({
    queued: mac.queued,
  });
}

const now = () => new Date().toISOString();
let made = 0;
const rowId = (id: string) => `${id}-${Date.now()}-${made++}`;

/** The Job's list row, brought in line with its thread: what it waits on, whether a turn runs, and the Mac's Needs you. */
function settle(mac: FixtureMac, thread: SessionThread): void {
  const record = thread.session;
  const at = mac.live.findIndex((one) => one.id === record.id);
  if (record.state === "ended") {
    if (at >= 0) mac.live.splice(at, 1);
  } else if (at >= 0) {
    const was = mac.live[at]!;
    const waits = record.waits !== undefined;
    const waiting =
      !waits && ((record.waiting_for ?? []).length > 0 || record.hosted?.asked !== undefined);
    const next: PhoneSession = {
      ...was,
      held: record.hosted?.start !== undefined,
      working: record.hosted?.turn.state === "working",
      waiting,
    };
    if (waiting) next.waiting_since = was.waiting_since ?? now();
    else delete next.waiting_since;
    if (!waits) delete next.starts_when;
    mac.live[at] = next;
  }
  mac.needs = mac.live.filter((one) => one.waiting);
}

const hosted = (record: SessionRecord) =>
  (record.hosted ??= {
    turn: {
      state: "idle",
    },
    mode: "auto",
    running: true,
  });
const drop = (record: SessionRecord, item: string) =>
  (record.waiting_for = (record.waiting_for ?? []).filter((one) => one.id !== item));
const push = (thread: SessionThread, row: SessionRow) => thread.rows.push(row);

/** `answer_items`: each item taken off its Job's list, or refused as Fleet refuses one it no longer waits on. */
function itemsAnswered(mac: FixtureMac, sent: AnswerItems): Answered {
  return json({
    outcomes: sent.items.map((one) => {
      const thread = mac.threads[one.session_id];
      const held = (thread?.session.waiting_for ?? []).some((item) => item.id === one.item);
      if (thread === undefined || !held)
        return {
          session_id: one.session_id,
          item: one.item,
          outcome: {
            kind: "refused",
            code: "fleet.items_not_waiting",
            message: "The Job no longer waits on this.",
          },
        };
      // Fleet's `answering_items.rs`: an agent's item approves only when raised with `approves`, and a send-back says what to change.
      const waiting = (thread.session.waiting_for ?? []).find((item) => item.id === one.item)!;
      const refused = (code: string, message: string) => ({
        session_id: one.session_id,
        item: one.item,
        outcome: {
          kind: "refused" as const,
          code,
          message,
        },
      });
      if (one.how === "approve" && waiting.source === "agent" && waiting.approves !== true)
        return refused(
          "fleet.items_not_approvable",
          `"${waiting.text}" has no Approve: it is answered on its own`,
        );
      if (one.how === "send_back" && (one.notes ?? []).every((note) => note.text.trim() === ""))
        return refused(
          "fleet.items_send_back_empty",
          `"${waiting.text}" is sent back with a note saying what to change`,
        );
      drop(thread.session, one.item);
      settle(mac, thread);
      return {
        session_id: one.session_id,
        item: one.item,
        outcome: {
          kind: "settled",
        },
      };
    }),
  });
}

/** What each Job act does to the fixture, as Fleet would. Answers undefined for 204 with no body. */
function act(mac: FixtureMac, path: string, sent: Record<string, unknown>): Answered | undefined {
  const id = String(sent["session_id"] ?? sent["waiting_session"] ?? "");
  const thread = mac.threads[id];
  if (!path.startsWith("/api/sessions/") || path === "/api/sessions/answer") return undefined;
  if (thread === undefined) return said(404, "No Job has that id on this Mac.");
  const record = thread.session;
  const job = hosted(record);
  const answered = (): Answered => {
    settle(mac, thread);
    return json(record);
  };
  switch (path) {
    case "/api/sessions/begin": {
      if (job.start === undefined) return said(409, "This Job is not on its start card.");
      // A Job that was told to press Start on this one stops waiting for it.
      for (const other of Object.values(mac.threads)) drop(other.session, `start:${id}`);
      if (job.start.asks_delivery && sent["delivery"] === undefined)
        return said(422, "Choose how the work lands.");
      const message =
        typeof sent["message"] === "string" ? sent["message"] : (job.start?.message ?? "");
      delete job.start;
      drop(record, "first_message");
      job.turn = {
        state: "working",
      };
      job.running = true;
      push(thread, {
        kind: "message",
        id: rowId(id),
        at: now(),
        from: {
          kind: "you",
        },
        text: message,
      });
      return answered();
    }
    case "/api/sessions/change_waiting": {
      if (record.waits === undefined) return said(409, `job ${id} does not wait.`);
      const change = sent as ChangeWaiting;
      if (change.held !== undefined) record.waits.held = change.held;
      if (change.condition !== undefined) {
        record.waits.condition = change.condition;
        delete record.waits.part_title;
        if (change.condition.kind === "group_done" || change.condition.kind === "task_done") {
          const part =
            change.condition.kind === "group_done" ? change.condition.group : change.condition.task;
          const groups = mac.threads[record.waits.on.id]?.session.plan?.groups ?? [];
          record.waits.part_title =
            groups.find((one) => one.id === part)?.title ??
            groups.flatMap((one) => one.tasks).find((one) => one.id === part)?.text ??
            part;
        }
      }
      return answered();
    }
    case "/api/sessions/start_waiting": {
      if (record.waits === undefined) return said(409, `job ${id} does not wait.`);
      const { held, brief } = record.waits;
      delete record.waits;
      if (held)
        job.start = {
          asks_delivery: true,
          message: brief,
        };
      else {
        job.turn = {
          state: "working",
        };
        push(thread, {
          kind: "message",
          id: rowId(id),
          at: now(),
          from: {
            kind: "you",
          },
          text: brief,
        });
      }
      return answered();
    }
    case "/api/sessions/message":
      push(thread, {
        kind: "message",
        id: rowId(id),
        at: now(),
        from: {
          kind: "you",
        },
        text: String(sent["text"] ?? ""),
      });
      job.turn = {
        state: "working",
      };
      return answered();
    case "/api/sessions/stop":
      job.turn = {
        state: "idle",
      };
      return answered();
    case "/api/sessions/pause":
      job.turn = {
        state: "idle",
      };
      job.paused = {
        at: now(),
        resuming: false,
      };
      return answered();
    case "/api/sessions/resume":
      delete job.paused;
      return answered();
    case "/api/sessions/close":
      record.state = "ended";
      record.ended_at = now();
      job.turn = {
        state: "idle",
      };
      return answered();
    case "/api/sessions/tune": {
      const tune = sent as TuneSession;
      if (tune.model !== undefined) job.model = tune.model;
      if (tune.effort !== undefined) job.effort = tune.effort;
      job.mode = tune.mode;
      return answered();
    }
    case "/api/sessions/run/start": {
      const command = String(sent["command"] ?? "");
      // The item stays: on a Mac the agent clears it once it reads the run's end.
      push(thread, {
        kind: "ran",
        id: rowId(id),
        at: now(),
        command,
        code: 0,
        tail: "Merged 3 branches into preview.\nFleet restarted on preview-4e735d6.\nBridge reconnected.",
      });
      settle(mac, thread);
      return json(
        {
          run_id: rowId("run"),
        },
        202,
      );
    }
    case "/api/sessions/sketch/answer":
    case "/api/sessions/document/answer": {
      const sketch = path.includes("/sketch/");
      const named = String(sent[sketch ? "sketch" : "document"] ?? "");
      const target = sketch ? named : named.replace(/^document:/, "");
      const held = record.attachments.find((one) => one.target === target);
      const detail = held?.detail;
      const refuse = (status: number, code: string, message: string) =>
        json(
          {
            code,
            message,
          },
          status,
        );
      if (detail === undefined || detail["asked"] !== "true" || detail["settled"] !== undefined)
        return refuse(
          422,
          sketch ? "fleet.sketch_not_open" : "fleet.document_not_open",
          "that page is not waiting on an answer",
        );
      // The job draws it again after he read it: the new drawing is in the thread, and this answer is refused.
      const again = sketch ? mac.redraws?.[`${id} ${target}`] : undefined;
      if (again !== undefined) {
        delete mac.redraws?.[`${id} ${target}`];
        Object.assign(detail, again.detail);
        push(thread, {
          kind: "tool",
          id: rowId(id),
          at: now(),
          text: again.call,
        });
        settle(mac, thread);
        return refuse(
          409,
          "fleet.sketch_redrawn",
          "the sketch is not the drawing that was read: the job has drawn it again since",
        );
      }
      if (sketch ? sent["drawn"] !== detail["drawn"] : sent["revision"] !== detail["revision"])
        return refuse(
          409,
          sketch ? "fleet.sketch_redrawn" : "fleet.document_revised",
          "that page changed since it was read",
        );
      const how = sent["how"];
      const words = typeof sent["words"] === "string" ? sent["words"] : undefined;
      if (how === "sent_back" && (words === undefined || words.trim() === ""))
        return refuse(
          422,
          sketch ? "fleet.sketch_answer_empty" : "fleet.document_answer_empty",
          "sending back takes a note",
        );
      detail["settled"] = String(how);
      detail["settled_at"] = now();
      if (words !== undefined) detail["words"] = words;
      drop(record, named);
      if (how === "sent_back")
        job.turn = {
          state: "working",
        };
      return answered();
    }
    case "/api/sessions/artifacts/part/answer":
      return partAnswered(mac, record, sent as unknown as AnswerPart);
    case "/api/sessions/artifacts/press":
      return json({
        revision: Number(sent["revision"] ?? 1),
        action: String(sent["action"] ?? ""),
        act: "message",
        at: now(),
        did: {
          kind: "sent",
        },
      });
    case "/api/sessions/artifacts/vote":
      return json({
        revision: Number(sent["revision"] ?? 1),
        vote: String(sent["vote"] ?? ""),
        form: "pick",
        result: sent["result"],
        at: now(),
      });
    case "/api/sessions/artifacts/seen":
      return json({
        seen: Number(sent["revision"] ?? 1),
      });
    case "/api/sessions/ready_to_try/answer": {
      const given = sent as unknown as AnswerReadyToTry;
      const card = (mac.tries?.[id] ?? []).find(
        (one) => one.branch === given.branch && one.tip === given.tip,
      );
      if (card === undefined)
        return json(
          {
            code: "fleet.ready_to_try_unknown",
            message: "no card of that branch and tip",
          },
          404,
        );
      if (card.state === "answered")
        return json(
          {
            code: "fleet.ready_to_try_settled",
            message: "that card was answered already",
          },
          409,
        );
      if (given.answer.kind === "remind") {
        card.state = "later";
        card.later_until = given.answer.until;
      } else {
        card.state = "answered";
        card.verdict = given.answer.kind === "looks_good" ? "good" : "needs_work";
      }
      drop(record, card.id);
      settle(mac, thread);
      return json(card);
    }
    case "/api/sessions/waiting/done":
      drop(record, String(sent["item_id"] ?? ""));
      return answered();
    case "/api/sessions/waiting/answer":
    case "/api/sessions/waiting/dismiss": {
      const given = sent as AnswerWaiting & DismissWaiting;
      drop(record, given.item_id);
      const draft = given.item_id.startsWith("draft:")
        ? given.item_id.slice("draft:".length)
        : undefined;
      thread.rows = thread.rows.map((row) => {
        if (row.kind !== "draft" || row.id !== draft) return row;
        const how = path.endsWith("dismiss")
          ? "withdrawn"
          : row.change !== undefined && given.choice !== undefined && given.choice !== 0
            ? "withdrawn"
            : given.fixed !== undefined
              ? "fixed"
              : given.text !== undefined
                ? "sent_back"
                : "approved";
        const words = given.text ?? given.reason;
        return {
          ...row,
          ...(given.fixed === undefined
            ? {}
            : {
                text: given.fixed,
              }),
          settled: {
            how,
            ...(words === undefined
              ? {}
              : {
                  words,
                }),
            at: now(),
          },
        };
      });
      if (draft !== undefined && !path.endsWith("dismiss"))
        job.turn = {
          state: "working",
        };
      return answered();
    }
    case "/api/sessions/fork": {
      const fork = `${id}-fork-${made++}`;
      const title = record.title ?? "Job";
      mac.threads[fork] = {
        session: {
          ...record,
          id: fork,
          started_at: now(),
          started: {
            by: {
              id,
              title,
            },
          },
          waiting_for: [],
          hosted: {
            ...job,
            turn: {
              state: "idle",
            },
          },
        },
        rows: thread.rows.filter((row) => row.kind === "message"),
      };
      thread.rows.push({
        kind: "started",
        id: rowId(id),
        at: now(),
        session: {
          id: fork,
          title,
        },
        fork: true,
      });
      const listed = mac.live.find((one) => one.id === id);
      const session: PhoneSession = {
        ...(listed ?? {
          kind: "hosted",
          started_at: now(),
        }),
        id: fork,
        title,
        held: false,
        waiting: false,
        working: false,
        started_at: now(),
        from: {
          id,
          title,
        },
      };
      delete session.waiting_since;
      mac.live.unshift(session);
      settle(mac, thread);
      return json(
        {
          session,
        },
        201,
      );
    }
  }
  return undefined;
}

/** `answer_part`, as Fleet answers it: the part answered in place, the artifact settled once no ask is left, or Fleet's refusal. */
function partAnswered(mac: FixtureMac, record: SessionRecord, given: AnswerPart): Answered {
  const refuse = (status: number, code: string, message: string) =>
    json(
      {
        code,
        message,
      },
      status,
    );
  const artifact: JobArtifact | undefined = mac.artifacts?.[record.id]?.find(
    (one) => one.id === given.artifact,
  );
  const part = artifact?.parts?.find((one) => one.id === given.part);
  if (artifact === undefined || part === undefined)
    return refuse(422, "fleet.part_refused", "that artifact holds no such part");
  if (!part.asks || part.state !== "open")
    return refuse(422, "fleet.part_refused", "that part is not waiting on an answer");
  if (part.drawn !== given.drawn)
    return refuse(409, "fleet.part_redrawn", "the job has put that part again since");
  if (given.how === "sent_back" && (given.words ?? "").trim() === "")
    return refuse(422, "fleet.part_refused", "sending back takes a note");
  if ((given.how === "picked") !== (part.kind === "takes"))
    return refuse(422, "fleet.part_refused", "a takes part is picked, and only a takes part");
  part.state = given.how;
  part.answer = {
    ...(given.words === undefined
      ? {}
      : {
          words: given.words,
        }),
    ...(given.take === undefined
      ? {}
      : {
          take: given.take,
        }),
    ...(given.notes === undefined
      ? {}
      : {
          notes: given.notes,
        }),
    at: now(),
  };
  const parts = artifact.parts ?? [];
  if (!parts.some((one) => one.asks && one.state === "open")) {
    artifact.state = parts.some((one) => one.state === "sent_back") ? "sent_back" : "approved";
    delete artifact.item;
    drop(record, artifact.id);
    const thread = mac.threads[record.id];
    if (thread !== undefined) settle(mac, thread);
  }
  return json(artifact);
}
