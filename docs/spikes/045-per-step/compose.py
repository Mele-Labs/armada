#!/usr/bin/env python3
"""The first message of one stretch: its instructions from bug.json, the report, the rig's notes
and the previous agent's handoff.

  python3 -I compose.py --yardstick FILE --bug-json FILE --stretch 1|2|3 [--handoff FILE]
                        [--branch NAME] [--test-cmd CMD] [--retry-note FILE]
"""
import argparse
import json
import re

STRETCHES = {
    1: dict(
        label="Find the cause and plan the fix",
        steps=["find_the_cause"],
        ends="the person approves your plan",
    ),
    2: dict(
        label="Fix, show the test red then green, push",
        steps=["fix", "tests"],
        ends="the yardstick's test file passes at the commit you push to `origin` (the runner runs it itself, once you have called all_done)",
    ),
    3: dict(
        label="Deliver",
        steps=["deliver"],
        ends="the person merges your branch into the `origin`'s main",
    ),
}


def bug_steps(path):
    text = open(path).read()
    body = "\n".join(l for l in text.splitlines() if not l.lstrip().startswith("#"))
    return {s["id"]: s for s in json.loads(body)["steps"]}


def parts(yardstick):
    text = open(yardstick).read()
    brief = text[text.index("## Brief") + len("## Brief"): text.index("## What the right answer looks like")].strip()
    report = brief[: brief.index("What I want:")].strip()
    howto = brief[brief.index("How to run the test."):].strip()
    return report, howto


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--yardstick", required=True)
    ap.add_argument("--bug-json", required=True)
    ap.add_argument("--stretch", type=int, required=True)
    ap.add_argument("--handoff")
    ap.add_argument("--branch", default="")
    ap.add_argument("--retry-note")
    a = ap.parse_args()
    s = STRETCHES[a.stretch]
    steps = bug_steps(a.bug_json)
    report, howto = parts(a.yardstick)
    out = []
    out.append(
        f"You are the agent for stretch {a.stretch} of 3 of one bug Job. A different agent did the earlier stretches, a different one does the later ones. "
        "You do only your stretch, and you stop when it is done. Other agents work in your absence and know only what the handoff below and the repository tell them."
    )
    out.append("## The report\n\n" + report)
    out.append(f"## Your stretch: {s['label']}\n\nIt ends when {s['ends']}.")
    for sid in s["steps"]:
        st = steps[sid]
        out.append(f"### {st['label']}\n\n{st['about']}\n\n{st['instructions']}")
    notes = [
        "There is no forge here: no pull request and no ci. `origin` is a local bare repository, and pushing a branch to it is the push.",
        "No cargo, no pnpm install, and no test file but the one named below: the machine is shared.",
    ]
    if a.stretch == 1:
        notes.append("Edit nothing in the repository. Running `armada run bootstrap` is not an edit.")
    if a.stretch in (2, 3):
        if a.branch:
            notes.append(
                f"The work is on the branch `{a.branch}`. Your own Job gets a fresh branch when you first write, so your first act in your slot is `git checkout {a.branch}`. "
                f"Work, commit and push that branch (`git push origin {a.branch}`), not the Job's own."
            )
        else:
            notes.append("Work on the branch your own Job is given when you first write, and push it to `origin`.")
    if a.stretch == 3:
        notes.append(
            "In place of the pull request and the merge on a green ci: bring main into the branch (`git fetch origin` and `git merge origin/main`), run the test once more, push the branch, "
            f"then call waiting_for with exactly one item whose text is `Merge {a.branch or 'the branch'} into main`, and wait. The person merges the branch into the `origin`'s main and answers the item. "
            "Then say what landed, in your reply, and call all_done. Plan nothing new and change no code beyond what bringing main in needs."
        )
    out.append("## In this rig\n\n" + "\n".join(f"- {n}" for n in notes) + "\n\n" + howto)
    if a.stretch == 1:
        out.append("## Handoff in\n\nNone. You are first.")
    else:
        h = open(a.handoff).read().strip() if a.handoff else "(the previous agent left none)"
        out.append("## Handoff in, written by the previous agent\n\n" + h)
        if a.retry_note:
            out.append("## The runner's check of the previous stretch\n\n" + open(a.retry_note).read().strip())
    if a.stretch < 3:
        close = (
            "## When your stretch is done\n\n"
            + ("Fleet tells you when the person approves the plan. " if a.stretch == 1 else "")
            + "Your last message before you call all_done is the handoff brief for the next agent. "
            "Its first line is `HANDOFF`. Then three short sections: where the work is (branch, commit, files, what you ran), what you decided and why, and what is open. "
            "Write it for an agent that has not seen this conversation and starts from it alone. Then call all_done."
            + (" Do not begin the fix." if a.stretch == 1 else "")
        )
        out.append(close)
    print("\n\n".join(out))


main()
