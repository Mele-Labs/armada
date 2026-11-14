// A Job's sketches, drafts and pages as one record. `crates/ipc/src/artifact.rs`,
// `docs/practices/protocol/a-jobs-artifacts-as-one-record.md`, `docs/concepts/artifact.md`.
//
// The header rules in `protocol.ts` hold here. Nothing is published: Bridge reads again on a `session.changed`,
// which an archive, a restore, his Edit, a press, a vote and a mark that moved `seen` publish.

/** What an artifact is. A page is a document under `.armada/documents/`. */
export type ArtifactForm = "sketch" | "draft" | "page";

/** Who wrote a revision, `owner` for his Edit, or who archived an artifact. */
export type ArtifactBy = "agent" | "owner";

/** Who folded an artifact away, and when. Only `restore_artifact` lifts it; a new revision does not. */
export type ArtifactArchived = {
  by: ArtifactBy;
  at: string;
};

/**
 * `POST /sessions/artifacts/archive` and `POST /sessions/artifacts/restore`, each answered with the `JobArtifact` as it
 * now reads. Bridge's press archives as `owner`. 422 `fleet.artifact_not_held` for an id the Job does not hold, naming
 * at most 20 it does; `fleet.artifact_archived_already` and `fleet.artifact_not_archived`; `fleet.artifact_unnamed`
 * and `fleet.session_unnamed` for a Job not named or not known.
 */
export type ArtifactRef = {
  /** The artifact's `id`. */
  artifact: string;
  session_id?: string;
};

/**
 * `POST /sessions/artifacts/edit`: his Edit of a page's or a draft's words, answered with the `JobArtifact` as it now
 * reads, its newest revision `by` `owner`. The Job is sent each change first. A page's `text` is its whole HTML, the same
 * elements and attributes in the same order, with words changed only in text, headings, lists and table cells; a
 * draft's is its markdown. 409 `fleet.edit_not_newest`, naming the newest; 422 `fleet.edit_not_words` for a sketch or
 * an edit of anything but words, naming the first element that differs; `fleet.edit_settled`, `fleet.edit_archived`,
 * `fleet.artifact_not_held` and `fleet.session_unnamed`. 500 `fleet.edit_unkept`.
 */
export type SaveArtifactEdit = {
  session_id: string;
  /** The artifact's `id`: the page's path or `draft:<title>`. */
  artifact: string;
  /** The revision he edited, which must be the newest. */
  revision: number;
  text: string;
};

/** Where an artifact's newest revision stands. */
export type ArtifactState =
  | "reading"
  | "asking"
  | "approved"
  | "sent_back"
  | "withdrawn"
  | "dismissed";

/** One revision of an artifact. */
export type ArtifactRevision = {
  /** 1 and on, in the order they were kept. */
  number: number;
  /** When it was kept. */
  at: string;
  by: ArtifactBy;
  form: ArtifactForm;
};

/**
 * `GET /sessions/artifacts?session_id=` answers a list of these, newest first by when the newest revision was kept.
 * 422 `fleet.session_unnamed` for a Job Fleet does not know.
 */
export type JobArtifact = {
  /** Stable across revisions: `sketch:<title>`, the page's path, or `draft:<title>`. */
  id: string;
  /** The newest revision's title. */
  title: string;
  /** The newest revision's form. */
  form: ArtifactForm;
  /** Oldest first. Never empty. */
  revisions: ArtifactRevision[];
  state: ArtifactState;
  /** His note on a send back, his words with an approval, or his reason with a draft's dismissal. */
  words?: string;
  /**
   * The Needs you item that answers it now: `sketch:<title>`, `document:<path>`, `draft:<row id>`, or `plan:<version>`
   * for a sketch the pending plan names. Absent while an archive keeps it off his list.
   */
  item?: string;
  /** The plan version a sketch goes with: the pending one naming it, or the one whose Approve settled it. */
  in_plan?: number;
  /** Set while archived. `state` still reads as it stood. */
  archived?: ArtifactArchived;
  /** What the newest revision is for, and what it links to. */
  for_line?: ArtifactFor;
  /** Every press of an action a page declares, oldest first. Absent where none, and for every form but a page. */
  presses?: PagePress[];
  /** The newest cast of each vote a page declares, in the order first cast. Absent where none, and for every form but a page. */
  votes?: VoteCast[];
  /** The newest revision he opened, as `mark_artifact_seen` kept it. Absent while he opened none. Only moves forward. */
  seen?: number;
};

/**
 * `POST /sessions/artifacts/seen`: he opened revision `revision` of an artifact. Answered with `ArtifactSeen`. An
 * older revision is answered with the current `seen`, not refused. Nothing is sent to the Job. 422
 * `fleet.artifact_revision_unknown` for a revision never kept, `fleet.artifact_not_held` and `fleet.session_unnamed`.
 */
export type MarkArtifactSeen = {
  session_id: string;
  /** The artifact's `id`. */
  artifact: string;
  revision: number;
};

/** The newest revision he has opened, after the mark. */
export type ArtifactSeen = {
  seen: number;
};

/** The forms a vote takes. */
export type VoteForm = "pick" | "rank" | "score";

/**
 * `POST /sessions/artifacts/vote`: his vote on an `<armada-vote>` on a page, read off the kept revision he voted
 * on. Answered with the `VoteCast`. 422 `fleet.page_vote_unknown`, `fleet.page_vote_form`, `fleet.page_vote_takes`,
 * `fleet.page_archived`, `fleet.artifact_not_held` and `fleet.session_unnamed`.
 */
export type CastPageVote = {
  session_id: string;
  /** The page's artifact `id`: its path. */
  artifact: string;
  /** The kept revision he voted on. */
  revision: number;
  /** The vote's `id` attribute. */
  vote: string;
  result: VoteResult;
};

/** What he voted, tagged by `form`. `notes` are his words on a take, by the take's id. */
export type VoteResult =
  | {
      form: "pick";
      take: string;
      notes?: Record<string, string>;
    }
  | {
      form: "rank";
      /** Every take once, first place first. */
      order: string[];
      notes?: Record<string, string>;
    }
  | {
      form: "score";
      /** A part left out is no vote. */
      marks: VoteMark[];
      notes?: Record<string, string>;
    };

/** One keep or drop on one named part of one take. */
export type VoteMark = {
  take: string;
  part: string;
  keep: boolean;
};

/** One cast of a page's vote, as Fleet kept it. */
export type VoteCast = {
  revision: number;
  vote: string;
  form: VoteForm;
  result: VoteResult;
  at: string;
};

/** The acts a page may run. Fleet keeps the list and refuses anything off it. */
export type PageAct = "message" | "start" | "answer";

/**
 * `POST /sessions/artifacts/press`: his press of an `<armada-action>` on a page, read off the kept revision he
 * pressed. Answered with the `PagePress`. 422 `fleet.page_action_unknown`, `fleet.page_act_refused`,
 * `fleet.page_ask_not_open`, `fleet.page_archived`, `fleet.artifact_not_held` and `fleet.session_unnamed`.
 */
export type PressPageAction = {
  session_id: string;
  /** The page's artifact `id`: its path. */
  artifact: string;
  /** The kept revision he pressed it on. */
  revision: number;
  /** The action's `id` attribute. */
  action: string;
  words?: string;
};

/** What a press did: the Job sent it, a Job started, or the Job's ask was answered. */
export type PressDid =
  | {
      kind: "sent";
    }
  | {
      kind: "started";
      job: string;
    }
  | {
      kind: "answered";
      item: string;
    };

/** One press of a page's action, as Fleet kept it. */
export type PagePress = {
  revision: number;
  action: string;
  /** The row or part it sits on. Absent where the page gave none. */
  item?: string;
  act: PageAct;
  at: string;
  did: PressDid;
};

/**
 * `GET /sessions/artifacts/page?session_id=&artifact=&revision=`: one kept revision of a page, the newest where
 * `revision` is left out, answered as the page itself, `text/html`, sealed: no network, its own scripts inert, and the
 * kit's script drawing its actions and votes. An archived page serves. 422 `fleet.artifact_not_held` for an artifact
 * not held or not a page, `fleet.artifact_revision_unknown` for a revision never kept, and `fleet.session_unnamed`.
 * `docs/practices/protocol/a-pages-sealed-serving.md`.
 */
export type GetArtifactPage = {
  session_id: string;
  /** The page's artifact `id`: its path. */
  artifact: string;
  revision?: number;
};

/** What the kit's script on a sealed page posts to its host: to `window.webkit.messageHandlers.armada` where the host has one (Native's WKWebView), otherwise to `window.parent` (Classic's iframe). The host makes the Fleet call with the Job's `session_id`. Rust's `PageMessage<PageToHost>`. */
export type PageToHost =
  | {
      armada: 1;
      kind: "press";
      artifact: string;
      revision: number;
      action: string;
      words?: string;
    }
  | {
      armada: 1;
      kind: "vote";
      artifact: string;
      revision: number;
      vote: string;
      result: VoteResult;
    };

/** What the host sends back to the sealed page for it to draw: Fleet's answer, or its refusal. Posted as a `message` event to Classic's iframe, or passed to `window.armadaHost` by Native. Rust's `PageMessage<HostToPage>`. */
export type HostToPage =
  | {
      armada: 1;
      kind: "pressed";
      press: PagePress;
    }
  | {
      armada: 1;
      kind: "voted";
      cast: VoteCast;
    }
  | {
      armada: 1;
      kind: "refused";
      /** The action pressed, or the vote cast, that was refused. */
      action?: string;
      vote?: string;
      code: string;
      message: string;
    };

/** What a link on a For line points at. `link` is the agent's address; the rest are Fleet's. */
export type ArtifactLinkKind = "plan_step" | "task" | "pull_request" | "link";

/** One link on a For line. */
export type ArtifactLink = {
  kind: ArtifactLinkKind;
  label: string;
  /** A plan group's id, a plan task's id, `#<n>`, or an `http(s)` address. */
  target: string;
};

/**
 * What an artifact is for: the agent's one sentence (at most 200 characters), its links, then the links Fleet knows
 * (the task in progress, its plan step, each open pull request the Job holds). Kept per revision.
 */
export type ArtifactFor = {
  text?: string;
  links: ArtifactLink[];
};

/** A link as the agent gives it to `show_sketch` or `propose_draft`: `http://` or `https://`, at most 5. */
export type ForLink = {
  label: string;
  url: string;
};

/** His verdict on one item with an Approve. */
export type ItemHow = "approve" | "send_back";

/**
 * `POST /sessions/items/answer`: his verdict on several items with an Approve, across Jobs, answered 200 with
 * `ItemsSettled` even where some are refused. Each settles as its own answer would, in order. `notes` go with that
 * item's verdict, numbered by Fleet; a `send_back` needs one. 422 only for a body that does not decode.
 */
export type AnswerItems = {
  items: ItemAnswer[];
};

export type ItemAnswer = {
  session_id: string;
  /** The `WaitingItem` id. */
  item: string;
  how: ItemHow;
  notes?: ItemNote[];
};

/**
 * One note of a verdict. A card's note box sends no `on`; a note on words of a page, or a part of a sketch, names
 * them, and the Job reads it as `On "<quote>": <text>`. A `quote` blank after trimming counts as no `on`.
 */
export type ItemNote = {
  text: string;
  on?: NoteOn;
};

/** The words a note points at, and the block they are in. */
export type NoteOn = {
  quote: string;
  /** The page block's id, or the sketch part's id. */
  block: string;
};

/**
 * `POST /sessions/items/dismiss`: several items with an Approve off his list with no verdict, answered as
 * `AnswerItems` is. Each Job is sent one message, a line per item.
 */
export type DismissItems = {
  items: ItemRef[];
};

export type ItemRef = {
  session_id: string;
  item: string;
};

/**
 * How one item ended. Refusal codes: `fleet.items_not_waiting`, `fleet.items_not_approvable`,
 * `fleet.items_send_back_empty`, `fleet.items_on_another_mac`, or the item's own answer's code.
 */
export type ItemSettled =
  | {
      kind: "settled";
    }
  | {
      kind: "refused";
      code: string;
      message: string;
    };

export type ItemOutcome = {
  session_id: string;
  item: string;
  outcome: ItemSettled;
};

/** One outcome per item asked, in the order asked. */
export type ItemsSettled = {
  outcomes: ItemOutcome[];
};
