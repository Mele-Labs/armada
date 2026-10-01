import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, waitFor } from "storybook/test";
import type { StagedAttachment } from "@armada/protocol";

import { DispatchSettings } from "../DispatchSettings/DispatchSettings";
import { GUIDE_DISPATCH } from "../../guides";
import { DispatchRequest } from "./DispatchRequest";

/**
 * Dispatching by describing the work, which is the only way in — the form
 * behind `Enter by hand` is gone, and Settings is where a decision is
 * overridden.
 *
 * The complaint this answers, in the owner's words: *"I hate having to come up
 * with a title and brief and determine the workflow for a set of work."* Three
 * fields, and the Job proposer answers all three off one reading.
 *
 * **Every state here is one a person meets before the press.** The press
 * leaves this card, and the wait is drawn on the Job the press created —
 * `Proposer wait`'s stories.
 */
const meta: Meta<typeof DispatchRequest> = {
  title: "Compositions/Dispatch request",
  component: DispatchRequest,
  args: {
    request: "",
    onRequest: fn(),
    repository: "/Users/user/armada",
    refs: { from: "main", target: "main" },
    branches: [
      { name: "main", base: true },
      { name: "armada/18-fold-the-capacity-read", job: "Fold the capacity read into one query" },
      { name: "armada/19-give-the-rail-its-own-scroll", job: "Give the rail its own scroll" },
    ],
    onRefs: fn(),
    links: [],
    onAddLink: fn(),
    onRemoveLink: fn(),
    onSearchFiles: fn(async () => []),
    attachments: [],
    onStage: fn(async () => ({ path: "/tmp/staged" })),
    onAttach: fn(),
    onRemoveAttachment: fn(),
    onDispatch: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof DispatchRequest>;

/** The request one of these stories was written from, kept in one place. */
const REQUEST =
  "The board flickers every time an event lands. Find out why and stop it — it has been " +
  "doing it since the resync change.";

/**
 * Nothing typed. The one control that spends money is off, and the field says
 * what it takes: prose, or a link.
 *
 * **The button is off rather than absent.** A control that appears once the
 * field is filled teaches nothing about what the surface is for; one that is
 * visibly off says a request is what it is waiting for.
 */
export const NothingTyped: Story = {
  /**
   * Typed into, it comes alive, and dispatching sends exactly once. A rendering
   * shows the button greyed; only a press shows that nothing went out.
   */
  play: async ({ args, canvas, userEvent, step }) => {
    await step("the control that spends money is off until a request exists", async () => {
      const dispatch = canvas.getByRole("button", { name: "Dispatch" });
      await expect(dispatch).toBeDisabled();

      // Dispatched rather than clicked. The app's base styles take a disabled
      // control out of pointer reach, so a pointer cannot press it at all; the
      // event still arrives here to prove the handler is not bound either.
      fireEvent.click(dispatch);
      await expect(args.onDispatch).not.toHaveBeenCalled();
    });

    // #1540 drew the three as a numbered run under the controls, because the
    // order was the substance. Every one of them was true before anything was
    // typed, so #1602 took all three off and the card's own `?` is what
    // carries them — the whole of what this surface says about itself.
    await step("the card asks for the work and explains nothing about it", async () => {
      await expect(canvas.queryByText("What happens next")).toBeNull();
      await expect(canvas.queryByText(/Armada reads the request/)).toBeNull();
      await expect(canvas.queryAllByRole("listitem")).toHaveLength(0);
      await expect(
        canvas.getByRole("button", { name: `Open guide ${GUIDE_DISPATCH.number}, ${GUIDE_DISPATCH.title}` }),
      ).toBeVisible();
    });

    await userEvent.type(canvas.getByRole("textbox", { name: "Request" }), "Fix the flicker");
    await expect(args.onRequest).toHaveBeenCalled();
  },
};

/**
 * Typed, and ready. The same state as above with a request in it — the control
 * this surface exists for is live.
 */
export const Typed: Story = {
  args: { request: REQUEST },
  play: async ({ args, canvas }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(dispatch).toBeEnabled();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(dispatch);
    await expect(args.onDispatch).toHaveBeenCalledOnce();
  },
};

/**
 * Typing `@` opens the mention popup, and picking a result inserts it into
 * the field — the way an editor's own file reference works.
 *
 * **A stateful wrapper, not static args.** Every other story here proves a
 * callback fired; this one proves what the field holds afterward, which needs
 * `onRequest` actually feeding back into `request` rather than a `fn()` that
 * drops it.
 */
export const MentionInserted: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md", "packages/components/README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@READ");
    await userEvent.click(await canvas.findByRole("option", { name: "README.md" }));
    await expect(field).toHaveValue("@README.md ");
  },
};

/**
 * Arrow keys move the active row without touching the field's text, and
 * `Enter` inserts whichever row that lands on — the keyboard path beside the
 * mouse `MentionInserted` above already proves.
 */
export const MentionChosenByKeyboard: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md", "packages/components/README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@");
    await canvas.findByRole("option", { name: "README.md" });

    await userEvent.keyboard("{ArrowDown}");
    await expect(
      canvas.getByRole("option", { name: "packages/components/README.md" }),
    ).toHaveAttribute("aria-selected", "true");

    await userEvent.keyboard("{Enter}");
    await expect(field).toHaveValue("@packages/components/README.md ");
  },
};

/**
 * `Escape` closes the popup without touching what was typed — the one way out
 * that leaves the `@` as plain text rather than turning it into a mention.
 */
export const MentionDismissedByEscape: Story = {
  render: (args) => {
    function Stateful() {
      const [request, setRequest] = useState("");
      return (
        <DispatchRequest
          {...args}
          request={request}
          onRequest={setRequest}
          onSearchFiles={() => Promise.resolve(["README.md"])}
        />
      );
    }
    return <Stateful />;
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    await userEvent.type(field, "@READ");
    await canvas.findByRole("option", { name: "README.md" });

    await userEvent.keyboard("{Escape}");

    await expect(canvas.queryByRole("listbox")).toBeNull();
    await expect(field).toHaveValue("@READ");
  },
};

/**
 * An `@` that does not start a word — an email typed into the same field —
 * never opens the popup. `useMention`'s own note says why: an `@` preceded by
 * anything but whitespace stays plain text rather than a mention.
 */
export const MentionNotOpenedInsideAWord: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.type(canvas.getByRole("textbox", { name: "Request" }), "ping me at a@b");
    await expect(canvas.queryByRole("listbox")).toBeNull();
  },
};

/**
 * A screenshot pasted straight into the Request field, without a trip to the
 * file picker — `onRequestPaste`'s own path, proven the way `WithAttachments`
 * below proves the picker's: `onStage` and `onAttach` both fire with what the
 * paste carried.
 *
 * **A real `DataTransfer`, dispatched directly.** A real browser's
 * `ClipboardEvent` constructor requires `clipboardData` to be an actual
 * `DataTransfer` and throws otherwise, before the component ever sees the
 * paste. `fireEvent.paste` cannot carry it: `@testing-library/dom`'s
 * `createEvent` rebuilds `clipboardData` from `Object.getOwnPropertyNames`
 * of whatever is passed — a jsdom-era shim — and a real `DataTransfer`'s
 * `items`/`files` live on its prototype, not as own properties, so that
 * rebuild silently produces an empty one. Dispatching the `ClipboardEvent`
 * ourselves is the seam that keeps the real data.
 */
export const PastedScreenshot: Story = {
  play: async ({ args, canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });
    const pasted = new File(["a screenshot"], "screenshot.png", { type: "image/png" });

    const dt = new DataTransfer();
    dt.items.add(pasted);
    field.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: dt }));

    // `stage()` reads the file's bytes before calling `onStage`, so the calls
    // land after this event handler returns — `waitFor` rather than a bare
    // assertion.
    await waitFor(() =>
      expect(args.onStage).toHaveBeenCalledWith(expect.anything(), "screenshot.png", "image/png"),
    );
    await expect(args.onAttach).toHaveBeenCalledWith({
      path: "/tmp/staged",
      filename: "screenshot.png",
      mimeType: "image/png",
    });
  },
};

/**
 * Plain text pasted into the Request is not read for images at all — it falls
 * through to the field as text, and nothing stages.
 */
export const PastedTextStagesNothing: Story = {
  play: async ({ args, canvas }) => {
    const field = canvas.getByRole("textbox", { name: "Request" });

    const dt = new DataTransfer();
    dt.setData("text/plain", "some text");
    field.dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: dt }));

    await expect(args.onStage).not.toHaveBeenCalled();
    await expect(args.onAttach).not.toHaveBeenCalled();
  },
};

/**
 * Files staged against the request, drawn as removable chips — the same
 * `AttachmentChip` the hand-entry form already uses, on this field instead.
 */
export const WithAttachments: Story = {
  args: {
    request: REQUEST,
    attachments: [
      { path: "/tmp/a", filename: "before.png", mimeType: "image/png" },
      { path: "/tmp/b", filename: "after.png", mimeType: "image/png" },
    ] satisfies StagedAttachment[],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Remove before.png" }));
    await expect(args.onRemoveAttachment).toHaveBeenCalledWith("/tmp/a");
  },
};

/**
 * Where the work starts and where it lands, differing — a Job cut from a
 * branch that is not merged yet, landing back in the long-lived one.
 *
 * **This is the case the pair exists for.** Read at a glance the two fields
 * look like one value repeated; here they are two answers.
 */
export const FromABranchThatIsNotWhereItLands: Story = {
  args: {
    request: REQUEST,
    refs: { from: "armada/18-fold-the-capacity-read", target: "main" },
  },
};

/**
 * A file, an address and the Studio node the request came off, with the
 * settings block beside them. The chips say which is which; the filename alone
 * could not.
 */
export const EverythingAttached: Story = {
  args: {
    request: REQUEST,
    attachments: [{ path: "/tmp/staged/flicker.png", filename: "flicker.png", mimeType: "image/png" }],
    links: ["armada/1162"],
    node: { name: "The Drones stat says nothing" },
    settings: (
      <DispatchSettings
        open
        onOpenChange={() => {}}
        settings={{ droneCap: 2 }}
        onSettings={() => {}}
        workflows={[]}
        models={["haiku", "sonnet", "opus"]}
        machineCap={4}
      />
    ),
  },
};

/**
 * Adding a link is two presses and a typed address, and the surface reports
 * the address rather than putting it in the request field itself — which is
 * what makes it a chip a person can take back.
 */
export const AddingALink: Story = {
  args: { request: REQUEST, onAddLink: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Add a link" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Link" }), "armada/1162");
    await userEvent.click(canvas.getByRole("button", { name: "Add" }));
    await expect(args.onAddLink).toHaveBeenCalledWith("armada/1162");
    await expect(args.onRequest).not.toHaveBeenCalled();
  },
};

/**
 * Write and Sketch, which are one request said two ways — #1547.
 *
 * **A `play`, because the whole claim is what survives the switch.** A still
 * shows one mode or the other; what has to hold is that the words typed under
 * Write come back untouched after a trip through Sketch, and that the picture
 * is attached in both.
 */
export const WriteOrSketch: Story = {
  args: {
    request: REQUEST,
    sketch: { name: "sketch 1" },
  },
  render: function Switching(props) {
    const [mode, setMode] = useState<"write" | "sketch">("write");
    const [request, setRequest] = useState(props.request);
    return (
      <DispatchRequest
        {...props}
        request={request}
        onRequest={setRequest}
        mode={mode}
        onMode={setMode}
        sketchPad={<p>The pad goes here.</p>}
      />
    );
  },
  play: async ({ canvas, userEvent, step }) => {
    const field = () => canvas.getByRole("textbox", { name: "Request" });

    await step("the picture is attached under Write, and the chip is its name alone", async () => {
      await expect(canvas.getByText("sketch 1")).toBeVisible();
      // Where it was made is the pad's line, and the pad is not open here —
      // the chip saying it too put `a Studio` on the screen twice.
      await expect(canvas.queryByText("From a Studio")).toBeNull();
    });

    await step("Sketch puts the pad where the field was", async () => {
      await userEvent.type(field(), " Start with the rail.");
      await userEvent.click(canvas.getByRole("tab", { name: "Sketch" }));
      await waitFor(() => expect(canvas.getByText("The pad goes here.")).toBeVisible());
      await expect(canvas.queryByRole("textbox", { name: "Request" })).toBeNull();
      // The chip stays: what is attached is a fact about the request, not
      // about which way of saying it is open.
      await expect(canvas.getByText("sketch 1")).toBeVisible();
    });

    await step("and Write brings the words back exactly as they were", async () => {
      await userEvent.click(canvas.getByRole("tab", { name: "Write" }));
      await waitFor(() => expect(field()).toBeVisible());
      await expect(field()).toHaveValue(`${REQUEST} Start with the rail.`);
    });
  },
};

/** A composer that takes words only draws no switch at all. */
export const NoSketchOffered: Story = {
  args: { request: REQUEST },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("tab", { name: "Sketch" })).toBeNull();
  },
};
