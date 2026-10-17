// The screens a Session's agent draws in the `screen-wireframes` walk: a desktop window, a web app in a browser frame, and a
// flow of two phone screens. A layout tree says what is on a screen and never where. Mock only: `show_sketch` hands Fleet the
// same shape, and `parseScene` checks it where it is drawn.

import type { ScreenElement } from "@armada/components";

import { arrow, scene, screen } from "../scenes";

const SESSIONS = ["Store clock reads", "Flaky store test", "Retry loop cap", "Release notes", "Slot pool sizes"];

const OPEN: ScreenElement = {
  type: "window",
  chrome: "desktop",
  title: "Sessions",
  children: [
    {
      type: "row",
      gap: "none",
      size: "fill",
      children: [
        { type: "nav", vertical: true, brand: "Work", items: SESSIONS, active: "Flaky store test" },
        {
          type: "nav",
          vertical: true,
          items: [{ icon: "message-square" }, { icon: "files" }, { icon: "git-branch" }, { icon: "clock" }, { icon: "settings" }],
          active: "message-square",
        },
        {
          type: "column",
          size: 3,
          gap: "sm",
          pad: "md",
          children: [
            {
              type: "chat",
              size: "fill",
              messages: [
                { from: "me", text: "Why does store_flaky fail near the hour?" },
                { from: "them", text: "The test reads the wall clock. It is pinned to a fixed instant now." },
                { from: "me", text: "Show me the change" },
                { from: "them", text: "Diff is in the Artifacts list." },
              ],
            },
            {
              type: "row",
              border: true,
              pad: "sm",
              gap: "sm",
              children: [
                { type: "badge", text: "Pinned" },
                { type: "text", text: "Inject the clock at open", size: "fill" },
                { type: "button", variant: "ghost", icon: "x" },
              ],
            },
            { type: "field", kind: "textarea", placeholder: "Message" },
            {
              type: "row",
              gap: "sm",
              children: [
                { type: "button", id: "artifacts", text: "Artifacts", icon: "paperclip", step: 1 },
                { type: "button", text: "Subagents", icon: "bot" },
                { type: "button", text: "Slot", icon: "layers" },
                { type: "button", text: "Checks", icon: "list-checks" },
                { type: "button", text: "Judge", icon: "check" },
                { type: "spacer" },
                { type: "button", text: "Send", variant: "primary", icon: "send" },
              ],
            },
          ],
        },
        {
          type: "column",
          size: 2,
          gap: "sm",
          pad: "sm",
          children: [
            { type: "panel", title: "Field", children: [{ type: "image", label: "Field" }] },
            {
              type: "panel",
              title: "Progress",
              pad: "none",
              size: "fill",
              children: [
                {
                  type: "list",
                  rows: [
                    { text: "Read the clock reads", icon: "check", meta: "done" },
                    { text: "Inject the clock", icon: "pencil", meta: "working", selected: true },
                    { text: "Pin it in the test", icon: "clock", meta: "next" },
                    { text: "Run the store tests", icon: "play", meta: "next" },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
    {
      type: "modal",
      id: "finder",
      title: "Artifact finder",
      step: 2,
      children: [
        { type: "field", kind: "search", placeholder: "Find an artifact" },
        {
          type: "list",
          rows: [
            { text: "Store clock reads", icon: "layers", meta: "sketch", selected: true },
            { text: "Pinned clock", icon: "layers", meta: "sketch" },
            { text: "flaky.rs", icon: "file-text", meta: "file" },
            { text: "Pull request 1843", icon: "git-branch", meta: "pull request" },
            { text: "Clock walk", icon: "globe", meta: "page" },
          ],
        },
      ],
    },
  ],
};

export const OPEN_TITLE = "Session open: less on screen, more on keys";
export const OPEN_SCENE = scene([screen("open", 0, 0, "Session", OPEN, { w: 1120 })], []);

const MEMBERS: ScreenElement = {
  type: "window",
  chrome: "browser",
  title: "Team settings",
  url: "app.tallyboard.example/settings/team",
  children: [
    { type: "nav", brand: "Tallyboard", items: ["Projects", "Reports", "Team"], active: "Team", actions: [{ icon: "search" }, { icon: "bell" }] },
    {
      type: "row",
      gap: "none",
      size: "fill",
      children: [
        {
          type: "nav",
          vertical: true,
          brand: "Settings",
          items: [
            { text: "Team", icon: "user-check" },
            { text: "Workspace", icon: "layout-grid" },
            { text: "Security", icon: "lock" },
            { text: "Integrations", icon: "link" },
            { text: "Notifications", icon: "bell" },
          ],
          active: "Team",
        },
        {
          type: "column",
          size: "fill",
          gap: "md",
          pad: "lg",
          children: [
            { type: "text", style: "heading", text: "Team" },
            { type: "tabs", items: ["General", "Members", "Billing"], active: "Members" },
            {
              type: "row",
              gap: "sm",
              children: [
                { type: "field", kind: "search", placeholder: "Find a member", size: "fill" },
                { type: "button", text: "Invite", variant: "primary", icon: "plus" },
              ],
            },
            {
              type: "table",
              columns: ["Name", "Email", "Role", "Last active"],
              rows: [
                ["Ada Okafor", "ada@northwind.example", "Owner", "Today"],
                ["Marcus Webb", "marcus@northwind.example", "Admin", "Today"],
                ["Priya Raman", "priya@northwind.example", "Member", "Yesterday"],
                ["Tomas Lindqvist", "tomas@northwind.example", "Member", "Last week"],
              ],
            },
            {
              type: "panel",
              title: "Invites",
              actions: [{ text: "Copy link", icon: "link" }],
              children: [
                {
                  type: "grid",
                  cols: 2,
                  gap: "md",
                  children: [
                    { type: "field", label: "Role for new members", kind: "select", value: "Member" },
                    { type: "field", label: "Link expires", kind: "select", value: "In seven days" },
                    { type: "field", label: "Require approval for invites", kind: "toggle", value: "on" },
                    { type: "field", label: "Tell admins when someone joins", kind: "checkbox", value: "on" },
                  ],
                },
                { type: "divider" },
                {
                  type: "row",
                  gap: "sm",
                  children: [{ type: "spacer" }, { type: "button", text: "Cancel" }, { type: "button", text: "Save changes", variant: "primary" }],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};

export const MEMBERS_TITLE = "Team settings";
export const MEMBERS_SCENE = scene([screen("members", 0, 0, "Settings, Members", MEMBERS)], []);

const SIGN_IN: ScreenElement = {
  type: "window",
  chrome: "phone",
  children: [
    {
      type: "column",
      size: "fill",
      gap: "md",
      pad: "lg",
      children: [
        { type: "spacer" },
        { type: "text", style: "heading", text: "Sign in", align: "center" },
        { type: "field", label: "Email", placeholder: "name@company.com" },
        { type: "field", label: "Password", placeholder: "Password" },
        { type: "field", kind: "checkbox", label: "Keep me signed in", value: "on" },
        { type: "button", id: "sign-in", text: "Sign in", variant: "primary", align: "stretch", step: 1 },
        { type: "text", style: "caption", text: "Forgot password", align: "center" },
        { type: "spacer" },
      ],
    },
  ],
};

const HOME: ScreenElement = {
  type: "window",
  chrome: "phone",
  children: [
    { type: "nav", brand: "Tallyboard", actions: [{ icon: "bell" }] },
    {
      type: "column",
      size: "fill",
      gap: "md",
      pad: "md",
      children: [
        { type: "text", style: "title", text: "Today" },
        {
          type: "list",
          rows: [
            { text: "Standup", icon: "clock", meta: "9:30" },
            { text: "Design review", icon: "clock", meta: "11:00", selected: true },
            { text: "Ship notes", icon: "clock", meta: "15:00" },
          ],
        },
      ],
    },
    {
      type: "nav",
      align: "stretch",
      items: [
        { text: "Home", icon: "house" },
        { text: "Tasks", icon: "list-checks" },
        { text: "Inbox", icon: "mail" },
        { text: "Settings", icon: "settings" },
      ],
      active: "Home",
    },
  ],
};

export const FLOW_TITLE = "Sign in flow";
export const FLOW_SCENE = scene(
  [screen("sign-in", 0, 0, "Sign in", SIGN_IN), screen("home", 600, 0, "Home", HOME)],
  [arrow("sign-in", "home", "Sign in", false, 2)],
);
