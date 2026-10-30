// `workspaces-pieces` with four Keep for every Job drafts open for store-web, each waiting on the owner's Approve: a rule and a lifecycle the repository holds, and a rule and a lifecycle it does
// not hold yet. The page marks the held two with the clock and draws the other two from the text of the file their approval would save. The walk `workflows-pending-keep` plays it.

import type { Scenario } from "../moment";
import { workspacesPieces } from "./workspaces-pieces";
import { workspacesLanes } from "./workspaces-lanes";

const [web] = workspacesLanes.state.holds.repositories ?? [];
const manifestId = web!.manifest!.id;

const startTheTicket = `version: 1
rule_id: progress
name: Start the ticket
when:
  enters:
    lifecycle: pull_request
    state: draft
do:
  - tracker_write:
      move: in_progress
      on:
        held: ticket
autonomy: asks
on_failure:
  retries: 2
  then: ask_owner
`;

const ticket = `version: 1
lifecycle_id: ticket
name: Ticket
scope: ${manifestId}
states:
  - id: todo
    label: To do
    read:
      tracker: [To Do, Backlog]
  - id: in_progress
    label: In progress
    read:
      tracker: [In Progress]
  - id: in_review
    label: In QA
    read:
      tracker: [QA]
  - id: blocked
    label: Blocked
    read:
      tracker: [Blocked]
  - id: done
    label: Done
    read:
      tracker: [Done, Closed]
transitions:
  - from: todo
    to: in_progress
  - from: in_progress
    to: in_review
  - from: in_review
    to: done
ends: [done]
`;

const moveToReview = `version: 1
rule_id: to-review
name: Move to review
scope: ${manifestId}
when:
  enters:
    lifecycle: pull_request
    state: ready
do:
  - tracker_write:
      move: in_review
      on:
        held: ticket
autonomy: acts
on_failure:
  retries: 2
  then: ask_owner
`;

const qaSignOff = `version: 1
lifecycle_id: qa_sign_off
name: QA sign-off
scope: ${manifestId}
states:
  - id: waiting
    label: Waiting
    read:
      tracker: [QA Waiting]
  - id: signed_off
    label: Signed off
    read:
      tracker: [QA Done]
transitions:
  - from: waiting
    to: signed_off
ends: [signed_off]
`;

export const workspacesPiecesKeeps: Scenario = {
  ...workspacesPieces,
  name: "workspaces-pieces-keeps",
  says: "The Workflows page over store-web's pieces, with four Keep for every Job drafts open: a held rule and lifecycle, and a rule and lifecycle it does not hold yet",
  draft: {
    pieces: {
      ...workspacesPieces.draft!.pieces!,
      keeps: [
        { manifestId, kind: "rule", id: "progress", text: startTheTicket },
        { manifestId, kind: "lifecycle", id: "ticket", text: ticket },
        { manifestId, kind: "rule", id: "to-review", text: moveToReview },
        { manifestId, kind: "lifecycle", id: "qa_sign_off", text: qaSignOff },
      ],
    },
  },
};
