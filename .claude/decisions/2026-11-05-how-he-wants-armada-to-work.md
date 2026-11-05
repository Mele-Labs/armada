# How does the owner want Armada to work, day to day?

**Said 2026-11-05.** He has given a lot of thoughts lately and wanted to be sure they had been heard. A planning Job quizzed him with moments and saved its predictions first. It missed most of them, so this file holds his answers rather than its reading of them.

This is direction, in his words. It builds nothing, and where it differs from a design he approved, that design stands until he says otherwise.

| The moment | He said | The cost in the option he took |
|---|---|---|
| A normal morning, with Armada working as he wants | "Talking to an operator to discuss work and identify what is currently going on. Jumping into individual jobs to review or assist with work as necessary" | His own wording |
| An agent pushes before he has approved its plan | Refuse the act | A one-line fix has to get a plan approved before it can push, narrowed by the next row |
| A Job with no plan fixes a one-line typo and pushes | Depends on size | Armada has to judge what counts as small, and sometimes it will judge wrong |
| A passing idea while he spitballs | Ask on the spot | It breaks his flow |
| A backend change he would never see, with green checks | "Again it depends on the project and that projects risk tolerance" | His own wording |
| What an agent runs on his Mac before pushing | "by default it should only run checks for what it changed". It depends on the repo and the project: "i might be working in an armada workspace/project that is one small area. And in that I am okay running all checks" | His own wording |
| A change he would see, with green checks | "Depends on the project. Also it needs to be smart. If I walked it already and approved a walk that was then just fully built out and refined I dont need to see the walk again unless the agent made some changes to the UI/UX based on the refinement" | His own wording |
| A Job goes quiet with work left and nothing asked | Armada fixes it quietly | He may not know it happened |
| Opening a Job to see how it is going | "Its plan and tasks plus what's running right now. Also not above the thread. I still like that its off to the right in its own pane/tabs" | His own wording |
| His feedback adds a whole new part to an approved plan | "Update the plan and keep going. In this case because its a UI/UX change that I had not seen before I will need to walk it to make sure it looks right (depending on the project)" | His own wording |

His no-nudging decision is about notices that argue with an agent. He takes a plain refusal.

## The concept with no name yet

"Depends on the project" came up in the answers on landing, local checks, walks, feedback and the Operator. He wants one thing that sets all of them: the area a project covers, its local checks, its risk tolerance, what needs a walk, and how the Operator behaves.

In his words: "sorry I dont know what we're calling this concept yet". And: "right now armada allows you to be in a state where you are seeing everything across projects or you can focus down to one project. I think we need a better space to configure everything about one project."

Neither of today's pieces does this. A Project ([Project](../../docs/concepts/project.md)) is a task list that spans repositories and knows nothing of risk or scope. A workspace ([Manifest](../../docs/concepts/manifest.md)) knows an area and its Checks and stays inside one repository.

## How far an Operator reaches: settled

Walking the panel on 5 Nov, he pointed at the repository picker and asked where an Operator's jurisdiction is: "I think this is a huge decision." Offered one Operator with its rules set per project, or one Operator per project, he chose **one Operator with rules per project**.

It sees every live Job on the Mac, so two Jobs on the same files are caught across projects. Its rules are set per project once the concept above exists, and until then its positions are set per repository. The title row's picker filters what the panel shows and changes nothing the Operator does.

Which rules live on a project is not ruled beyond what he named above. Roles, standing orders, the ceiling and the model are open, and so are whether a project's rules are set in this panel or in the space he asked for, and whether one repository can hold several projects.

## Where the Operator's open items stand

Walking the panel on 5 Nov: "I don't think the operator should have a typical job thread. I think that the job thread is where we can discuss but the open items for me and the operator should be in some queue tab that I can move around. Kind of like the \"Waiting to start\". I am imagining this queue is things that the operator is working through and then there is a queue of things that the operator is waiting on for me."

So Talk is the discussion, and a Queue tab holds what it is working through and what it waits on him for. Shown as a walk, not approved yet. He also chose a Queued mark on a message in both the panel and a Job's thread, and rows that drag as well as a tab that moves.

The order in that queue is his to set, and the Operator acts on it: "This is important. I want to be able to prioritize things and the operator understand that. So being able to drag and prioritize certain things is very important". And in words: "I should be able to tell the operator in words or through the UI to prioritize something and it can do what it needs to do to ensure that job is prioritized over others". Today a drag reaches only a held start, in its own repository's line; the Operator reads everything else in one call, in arrival order.

Walking it, he renamed the tab and the list: Talk should be "Operator" or "Discussion" (Discussion is drawn, since the panel is already titled Operator), and Working through is "Operators Queue". On a row: "I kind of feel like the items over here I should be able to take action on. if the operator is busy right now, I should have an action to say lets discuss this when you're ready. Otherwise other actions might go back into the operators queue or start right away dependning on what they are." On an item in the conversation: "Is this a message or a card that we are actively working through."

He keeps a way to start a Job himself: "I do think i need a way to be able to start a ad-hoc job. Sometimes I just jump into a job to dig into something or ask about something. I dont want to waste the operators time with this. The operator is aware of it and can monitor in case work starts that shouldn't".

## While he is away: not settled

Asked what the Operator should do when a Job asks which way to build something while he is away:

> This really depends on the project/repo(s) that I am working in and what type of build we're talking about. Is this a UI build that involves coming up with some UX or just a backend build. Is it a complex build or small build. what are the risks. It should realistically dispatch some kind of review of that work and then when I come back it should have sound proposals for me to select from. If it could not decide it should let me know so that I can jump into the job and start assisting directly.

Shown the Operator he approved on 4 Nov, he chose to talk it through first. Since #3296 each role stands at Off, Suggest or On, any role can differ per repository, and at Suggest a ruling waits for his one press. It does not yet dispatch a review or weigh a project's risk. **This file asks for no change to the Operator until he has talked it through.**

## The order he set, on 5 Nov

The sluggish app, then working out the concept above, then the Mac and test runs, then landing, then wrong notices. He placed the concept directly after the sluggish app; the rest kept their order around it. It is for getting Armada back to working smoothly, and he may change it.

He also turned workflows off on the Mac where they were still on, as he had on the other on 4 Nov. Rail A or B is still an open decision of his. Asked what he wants to see in a Job, he named the plan, tasks and what is running, not a step.

## Still open

What "size" means for a push, what the concept is called, and the talk about the Operator.
