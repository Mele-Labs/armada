# No small call moves to the on-device model yet

**Decision: none of Fleet's one-shot model calls is moved to Apple's on-device model for now.**
Two small calls, the quick-reply chips and the dispute read, are tried on it first, and nothing else is, until a trial says so.
Recommended after a measurement on 4 Nov 2026; the owner has not yet confirmed it.

## Why it was asked

Every one-shot call Fleet makes starts the full Claude CLI, about 300 MB and the owner's plugin hooks, and the account and the running summary alone may make one every 5 s a Session is at work.
`#3260` found that load to be part of what made his Mac chug.
He will not spend Claude tokens on these calls, so the on-device model was measured as the cheaper caller.

## What decided it

- **It is cheap.** A 92 KB Swift tool that reads a prompt on stdin and prints the answer took 22 MB and 0.1 s of CPU per call, with the first token at about 1 s, on macOS 26.5.2.
- **Fleet's readers accept what a person would not.** `read` and `read_so_far` in `crates/fleet/src/session_account.rs` keep 90% of the account answers by shape. Of 40 graded by hand, 7 were good.
- **The running summary copies its own prompt.** 79 of 80 answers repeated the example the prompt gives for the voice, and about 6 in 15 were good once the examples were removed.
- **The window is 4,096 tokens** for prompt and answer together. 11 of 30 real first requests overflowed it on the proposer, and the sites that carry diffs, files or the Logbook are out of reach by size.
- **The proposer picks a workflow rarely**: 2 to 4 of 14 requests, and `opus` nearly always.
- **A call with no response cap can run away**: 2 of 80 repeated until the window, one for 56 s. A cap of 200 tokens fixed both.
- **No guardrail trip or refusal** in about 300 calls, code-ish text included. That was not a reason to hold back.

## What is decided

| Call | Decision |
|---|---|
| The account and the running summary (`session_host/account.rs`) | Stay on Claude. They are the volume and the quality is not there. A shorter cadence is the lever on load, not a cheaper model |
| The Session proposer | Stays on Claude |
| Calls with tools, long input, a whole file or strict JSON: the Judge, review runs, retros, the Logbook, workflow changes, the test map, Autopilot | Stay on Claude |
| Quick-reply chips (`session_host/quick.rs`) and the dispute read (`session_host/dispute_reading.rs`) | Tried on the on-device model, with a response cap and a timeout, and a person looks at the answers before either moves |

## What would change it

- **A trial of the two** that shows their answers fit, judged by a person and not by Fleet's readers alone.
- **Guided generation** for the account and the summary, which was not tried and could fix the format but may not fix the content.
- **A prompt without examples**, and a reader that drops a `done:` line that only repeats a call.

## What adopting the two would take

- A second `ModelClient` whose `render` names the Swift tool, behind a router that sends only these sites to it, since Fleet holds one client for every site.
- The tool built on the Mac from the Command Line Tools, or shipped prebuilt and signed.
- An availability check, because the model can be off or not ready, and a fallback that keeps the last line and writes none, as no Claude tokens are spent.

## Not measured

Quality against Claude's (no Claude call was made), the quick chips and the dispute read on the model at all, the system process's own memory and energy, a first call after reboot, and whether the tool starts from Fleet's cleared environment.
