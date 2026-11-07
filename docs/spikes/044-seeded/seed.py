#!/usr/bin/env python3
"""Seed the bug workflow, regrouped by gates, into a Job's own task list.

  python3 -I seed.py --bug-json FILE --home SCRATCH_HOME --session ID --run-dir DIR

Writes ~/.claude/tasks/<session id>/<n>.json (the files Claude Code's TaskList reads) and
<run-dir>/template.json (which task is which gate, for gates.py and the timeline).

The stretches (each step's instructions are the lines of the matching step in bug.json, split or
trimmed as noted in the step's source comment below):

  stretch 1  1 Find the cause   2 Plan the fix                         3 GATE plan approved
  stretch 2  4 Fix   5 Tests (red, then green)   6 Push               7 GATE Checks green at the pushed commit
  stretch 3  8 Deliver                                                 9 GATE merged

Every task is blockedBy the one before it. A task is a Step (the agent's) or a Gate (Fleet's).
"""
import argparse
import json
import os


def bug_steps(path):
    text = open(path).read()
    body = "\n".join(l for l in text.splitlines() if not l.lstrip().startswith("#"))
    return {s["id"]: s["instructions"].split("\n") for s in json.loads(body)["steps"]}


def pick(lines, *idx):
    return "\n".join(lines[i] for i in idx)


def template(steps):
    cause, fix, tests, deliver = steps["find_the_cause"], steps["fix"], steps["tests"], steps["deliver"]
    # bug.json's find_the_cause is one step; the regrouped template splits its lines between two.
    find = pick(cause, 0, 2, 7, 8)
    plan = pick(cause, 1, 3, 4, 5, 6, 9, 10)
    # fix: bug.json's two walk lines are dropped, since nothing in these Jobs is seen in Bridge.
    fixing = pick(fix, 0, 1, 2, 3, 4)
    # tests: verbatim, plus a line saying what "the checks" are in this Job.
    testing = "\n".join(tests) + "\nIn this Job the repository's check is the one vitest file named in the brief."
    push = (
        "Commit in coherent pieces with messages that say why. Review reads only what is committed.\n"
        "Then push the branch to origin with git push origin <your branch>.\n"
        "Fleet runs the Checks itself at the commit you pushed. Do not mark the gate after this step: it is Fleet's."
    )
    # deliver: three of bug.json's lines (review, leave landable, report), and the delivery of this Job in place of its pull request lines.
    delivering = (
        pick(deliver, 2, 3, 7)
        + "\nDelivery in this Job: the branch is already pushed. Ask the person to merge it into the main line"
        " with ask_the_person, and stop until it is merged. Fleet ticks the last gate when it sees the merge on origin."
        " Then say what landed and call all_done."
    )

    def gate(subject, closes_when, then):
        return {
            "subject": subject,
            "activeForm": "Waiting for Fleet",
            "description": (
                f"Fleet's. It closes when Fleet sees {closes_when}. You do not mark it, and marking it moves nothing.\n"
                f"Do not start the next stretch before it is completed. {then}"
            ),
        }

    return [
        ("step", "Step: Find the cause", "Finding the cause", find),
        ("step", "Step: Plan the fix", "Planning the fix", plan),
        ("gate", gate("Gate: the person approves the plan. Fleet ticks this", "the person's approval of the recorded plan",
                      "After record_plan, wait: Fleet tells you when it is approved, and TaskGet on this task shows it.")),
        ("step", "Step: Fix", "Fixing", fixing),
        ("step", "Step: Tests, shown red then green", "Showing the test red then green", testing),
        ("step", "Step: Push the branch", "Pushing the branch", push),
        ("gate", gate("Gate: Checks green at the pushed commit. Fleet ticks this", "the Checks run green by Fleet at the commit on origin",
                      "If it stays open, the Checks are not green at what you pushed: read the file, fix, push again.")),
        ("step", "Step: Deliver", "Delivering", delivering),
        ("gate", gate("Gate: the branch is merged. Fleet ticks this", "the branch merged on origin",
                      "When it is completed, the Job is delivered.")),
    ]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bug-json", required=True)
    ap.add_argument("--home", required=True)
    ap.add_argument("--session", required=True)
    ap.add_argument("--run-dir", required=True)
    a = ap.parse_args()
    items = template(bug_steps(a.bug_json))
    d = f"{a.home}/.claude/tasks/{a.session}"
    os.makedirs(d, exist_ok=True)
    meta = []
    for n, item in enumerate(items, 1):
        if item[0] == "step":
            _, subject, active, description = item
        else:
            subject, active, description = item[1]["subject"], item[1]["activeForm"], item[1]["description"]
        task = {
            "id": str(n), "subject": subject, "description": description, "activeForm": active,
            "status": "pending", "blocks": [str(n + 1)] if n < len(items) else [], "blockedBy": [str(n - 1)] if n > 1 else [],
        }
        with open(f"{d}/{n}.json", "w") as f:
            json.dump(task, f, indent=2)
        meta.append({"id": str(n), "kind": item[0], "subject": subject})
    gates = {"plan_approved": "3", "checks_green": "7", "merged": "9"}
    json.dump({"session": a.session, "dir": d, "tasks": meta, "gates": gates}, open(f"{a.run_dir}/template.json", "w"), indent=2)
    print(f"seeded {len(items)} tasks in {d}")


if __name__ == "__main__":
    main()
