---
name: root-cause
description: Go from a failure to its cause — reproduce it first, find why it happens rather than where it surfaces, and fix it in the smallest change that addresses that cause. Load when a test, check, build or behaviour is failing and you have to make it right.
---

# Root cause

**A fix for a failure you never saw is a guess.** The repository's `CLAUDE.md`
or `AGENTS.md`, and its CI or testing practice doc if it has one, say how it
runs and reads its checks; they beat this file.

## Reproduce it first

- Run the same check, test or steps locally and see the same failure, with the
  same message.
- If it will not reproduce, find what differs — environment, data, order,
  timing — before changing any code. Say so if you cannot.

## Decide whose failure it is

If the same check is red on the default branch too, or the failure names code
or a service the change never touched, stop and report that. It is not this
branch's to fix.

## Find the cause, not the symptom

- Read the error from its first line, not its last. The first failure is
  usually the cause; the rest follow from it.
- Ask why it fails until the answer is something in the code or its inputs you
  can change: a wrong assumption, a missing case, a race, a stale value.
- **"Flaky" is not a cause.** A test that passes sometimes depends on time,
  order or shared state. Find which, and make it robust.

## Fix the cause, smallest first

- Change the least code that removes the cause. Leave neighbouring problems for
  your report.
- Run the same reproduction again and see it pass. Then run the checks around
  it, so the fix did not break a neighbour.

## What you never do

- Skip, disable, quarantine or loosen a test, or raise a timeout to hide a hang.
- Catch and swallow the error so the failure goes quiet.
- Push an empty commit to re-run CI and hope.

## Report

The failure, its cause in one line, the fix, and the run that shows it passing.
