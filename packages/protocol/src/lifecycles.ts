// Workspaces, lifecycles and rules, and where a Job's tickets, pull requests, deploys and started Jobs stand.
// Hand-mirrored from `crates/ipc/src/lifecycles.rs`. `docs/practices/protocol.md`, *Workspaces, lifecycles and rules, read*.
//
// The header rules in `protocol.ts` hold here: every closed set is left as `string`, with its words in the comment.
// **Reads only.** Nothing here saves a piece or moves an instance. Bridge draws a Job's lanes from `JobLifecycles` (`packages/screens/src/session-lanes.ts`); nothing draws the other reads yet.
//
// **A Workspace here is a set of repositories** (`WorkspaceOfRepositories`), not the Manifest's monorepo unit that
// `WorkspaceCommands` and `ScannedWorkspace` speak of.

import type { JobCopyChange } from "./job-copy";

/** Where a lifecycle or a rule applies, as its file says it. Every repository, or one Workspace or repository by id. */
export type PieceScope = { kind: "everywhere" } | { kind: "within"; id: string };

/** How a piece ranks against another of its id: the scope first, then the place, then whether it came from a Workspace's home. */
export type PieceRank = {
  /** How narrowly its scope names the repository asked about: `everywhere`, `workspace` or `repository`. */
  reach: string;
  /** The place it was kept: `armada`, `kit` or `repository`. */
  source: string;
  /** It came from the home repository of the Workspace it is scoped to. Absent is false. */
  home?: boolean;
};

/** Where a lifecycle or a rule was kept, and what became of it beside another of its id. */
export type PieceKept = {
  rank: PieceRank;
  /** The file it was read from. Absent for what Armada carries, which is built and not read. */
  path?: string;
  /** The rank of the one that runs instead of this. Absent on the one that runs. */
  overridden_by?: PieceRank;
  /** The place whose file this one replaced because it says something else: `armada`, `kit` or `repository`. */
  hides?: string;
};

/** A file set aside, and the sentence a person reads for it. `source` is `armada`, `kit` or `repository`. */
export type PieceLeftOut = { source: string; path: string; said: string };

/** A set of repositories that build one product, as one repository sees it. */
export type WorkspaceOfRepositories = {
  /** The id a lifecycle or a rule is scoped to it by. */
  id: string;
  name: string;
  /** Manifest ids, in the order the file lists them. */
  repositories: string[];
  /** The repository whose `.armada/workspaces/` holds the file, which the others read it from. Absent for one that is Kit's. */
  home?: string;
  /** The place it was kept: `kit` or `repository`. */
  source: string;
  path: string;
  /** The place of the one that runs instead of this. Absent on the one that runs. */
  overridden_by?: string;
  /** The place whose file this one replaced because it says something else. */
  hides?: string;
};

/** `list_repository_workspaces` (`GET /workspaces?manifest_id=`): the Workspaces that hold one repository. */
export type RepositoryWorkspaces = {
  manifest_id: string;
  /** The one that runs for each id, and each one it replaced. */
  workspaces: WorkspaceOfRepositories[];
  /** Every Workspace file Fleet could not read, whichever repository it was for. */
  left_out: PieceLeftOut[];
};

/** `list_workspaces` (`GET /workspaces/all`): every Workspace Fleet can see, whichever repositories it holds. */
export type AllWorkspaces = {
  /** Each file once: the one that runs for each id, and each one it replaced. Ordered by name. */
  workspaces: WorkspaceOfRepositories[];
  /** Every Workspace file Fleet could not read. */
  left_out: PieceLeftOut[];
};

/** How a state is read, by the source that knows. */
export type StateReadFrom =
  /** The tracker's own words for it. None is allowed, and every value is then unmapped. */
  | { source: "tracker"; words: string[] }
  /** What the forge shows of a pull request: `draft`, `ready`, `reviewers_on`, `in_review`, `approved`, `merged` or `closed`. */
  | { source: "forge"; fact: string }
  /** The state a watch's command printed: `waiting`, `running`, `passed` or `failed`. */
  | { source: "watch"; state: string }
  /** What Fleet knows without asking: `started`, `step_entered:<step id>`, `pull_request_opened`, `landed` or `ended`. */
  | { source: "fleet"; fact: string };

export type LifecycleStateDefinition = {
  id: string;
  /** What a person reads for it. */
  label: string;
  read: StateReadFrom;
};

/** A move a lifecycle expects. Expected, not permitted: a state is read and never set. */
export type LifecycleTransition = { from: string; to: string };

/** A lifecycle: the states one kind of thing passes through, the moves expected between them and the states that end it. */
export type LifecycleDefinition = {
  id: string;
  name: string;
  scope: PieceScope;
  states: LifecycleStateDefinition[];
  transitions: LifecycleTransition[];
  /** State ids. A Job stays open while an instance it holds is in none of them. */
  ends: string[];
};

/** A lifecycle that applies to a repository, and where it was kept. */
export type AppliedLifecycle = { lifecycle: LifecycleDefinition; kept: PieceKept };

/** `list_lifecycles` (`GET /lifecycles?manifest_id=`): what applies to one repository, and the files set aside. */
export type RepositoryLifecycles = {
  manifest_id: string;
  /** The one that runs for each id, and each one it replaced. */
  lifecycles: AppliedLifecycle[];
  left_out: PieceLeftOut[];
};

/** Which instances a rule looks at or acts on. */
export type RuleAmong =
  /** The instance whose entering a state set the rule off. */
  | { kind: "entered" }
  /** What the entered instance came from and the Job holds, of one lifecycle. */
  | { kind: "linked_from"; lifecycle: string }
  /** What came from the entered instance and the Job holds, of one lifecycle. */
  | { kind: "linking_to"; lifecycle: string }
  /** Every instance of one lifecycle the Job holds. */
  | { kind: "held"; lifecycle: string };

/** The instances an act is done to: those `among` selects, standing in `in_state` where one is given. */
export type RuleTarget = { among: RuleAmong; in_state?: string };

/** The moment a rule is for. `job`'s `event` is `started`, `pull_request_opened`, `landed` or `ended`. */
export type RuleWhen =
  | { kind: "enters"; lifecycle: string; state: string }
  | { kind: "step_starts"; step: string }
  | { kind: "step_ends"; step: string }
  | { kind: "job"; event: string };

/** One thing a rule asks. All of a rule's conditions must hold. `instances`' `quantity` is `every`, `any` or `no`. */
export type RuleCondition =
  | { kind: "instances"; among: RuleAmong; quantity: string; in_state: string }
  | { kind: "workflow"; workflow: string }
  | { kind: "at_step"; step: string }
  | { kind: "pull_request_opened" };

/** What a rule does. */
export type RuleAct =
  | { act: "tracker_assign"; to: string; target: RuleTarget }
  /** Move to a state of the ticket's lifecycle: the state, and not the tracker's word for it. */
  | { act: "tracker_move"; to: string; target: RuleTarget }
  | { act: "tracker_close"; target: RuleTarget }
  | { act: "forge_request_reviewers"; from: string[]; target: RuleTarget }
  | { act: "forge_mark_ready"; target: RuleTarget }
  | { act: "forge_merge"; target: RuleTarget }
  | { act: "forge_auto_merge"; target: RuleTarget }
  /** A message from a template, left on `target`. */
  | { act: "draft"; template: string; target: RuleTarget }
  | { act: "start_job"; workflow?: string; prompt: string }
  | { act: "start_watch"; recipe: string; values: Record<string, string>; deploy?: string }
  | { act: "tell"; message: string }
  | { act: "wake" }
  | { act: "run_command"; command: string }
  | { act: "run_skill"; skill: string }
  | { act: "run_drone"; prompt: string };

/** One act of a rule, and how far it goes on its own. */
export type RuleDoing = {
  what: RuleAct;
  /** The kind of act an autonomy default is set for: `tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`. */
  kind: string;
  /** What the rule does with it: `acts`, `asks` or `off`. The rule's own word, else the kind's default. */
  autonomy: string;
};

/** What a rule does when an act fails. */
export type RuleOnFailure = {
  /** How many times the act is tried again first. Capped. */
  retries: number;
  /** Whom a failure that retrying did not mend goes to: `tell_agent` or `ask_owner`. */
  then: string;
};

/** A lifecycle rule: When, If and Do, with an autonomy and a way to fail. Not `FleetRule`, the notice rule `fleet.toml` carries. */
export type LifecycleRule = {
  id: string;
  name: string;
  scope: PieceScope;
  when: RuleWhen;
  /** All of them must hold. None means the moment is enough. */
  conditions: RuleCondition[];
  does: RuleDoing[];
  /** The rule's own word, `acts`, `asks` or `off`, over the default for each kind of act. Absent takes the default. */
  autonomy?: string;
  on_failure: RuleOnFailure;
};

/** A rule that applies to a repository, and where it was kept. */
export type AppliedRule = { rule: LifecycleRule; kept: PieceKept };

/** `list_lifecycle_rules` (`GET /lifecycles/rules?manifest_id=`): what applies to one repository, and the files set aside. Armada carries none. */
export type RepositoryLifecycleRules = {
  manifest_id: string;
  rules: AppliedRule[];
  left_out: PieceLeftOut[];
};

/** Where an instance stands, as a source's last reading left it. `unmapped` is kept as the source said it and is not an end. */
export type InstanceStanding = { kind: "unread" } | { kind: "in"; state: string } | { kind: "unmapped"; said: string };

/** An instance another is linked to. */
export type InstanceLink = { lifecycle: string; key: string };

/** One thing a Job holds: a ticket, a pull request, a deploy, or a Job it started. */
export type HeldInstance = {
  lifecycle: string;
  /** What its source calls it: `ARM-41`, `812`, a Job's id. */
  key: string;
  /** What a person reads for it: the ticket's key, `PR #812`, the Job's title. */
  label: string;
  standing: InstanceStanding;
  /** What it came from, in the order the links were made. A link may name an instance the Job does not hold. */
  links: InstanceLink[];
  /** When ending the Job ended it, standing where it was. Absent while it is live, and for one that reached an end state of its own. */
  ended_at?: string;
};

/** One change of standing, kept when a read found the instance somewhere else. `source` is `tracker`, `forge`, `watch` or `fleet`. */
export type InstanceMove = {
  lifecycle: string;
  key: string;
  from: InstanceStanding;
  to: InstanceStanding;
  source: string;
  at: string;
};

/** One act a Job's rules planned, and where it stands. */
export type RuleActReading = {
  /** The id of the rule that planned it. */
  rule: string;
  /** The one line the person reads for it, as it was kept. */
  text: string;
  /** How far it goes on its own: `acts` or `asks`. */
  course: string;
  /** Where it stands: `planned`, `asked`, `stuck`, `done`, `skipped`, `told` or `not_carried`. */
  standing: string;
  /** What failed, how it was settled or why nothing here carries it. Absent where there is nothing to say. */
  said?: string;
  /** When it was planned. */
  at: string;
  /** When it was settled. Absent until it is. */
  settled_at?: string;
  /** The instance it is done to. Absent for an act done to none. */
  on?: InstanceLink;
  /** The instance whose entering a state set it off, where one did. */
  entered?: InstanceLink;
};

/**
 * `get_job_lifecycles` (`GET /sessions/lifecycles?session_id=`): what a Job holds and where it stands. A Job that holds
 * nothing answers the three lists empty. `lifecycles` are those its instances are of; `moves` are theirs, oldest first.
 */
export type JobLifecycles = {
  session_id: string;
  lifecycles: LifecycleDefinition[];
  instances: HeldInstance[];
  moves: InstanceMove[];
  /** The acts the Job's rules planned, oldest first. Absent where none was, and from a Fleet older than the field. */
  acts?: RuleActReading[];
  /** The rules the Job's copy holds, which are the ones that run for it. Absent for a Job that froze no copy, whose rules are the repository's. */
  rules?: AppliedRule[];
  /** What was changed in the Job's own copy, for the lanes to mark. Absent where nothing was. Since 30 Oct 2026. */
  changes?: JobCopyChange[];
};

/** `job_lifecycles.changed`: what a Job holds, whole, after an instance was taken on or moved. */
export type JobLifecyclesChanged = JobLifecycles;
