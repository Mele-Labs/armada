// Kit › Models over Fleet's answers: the offered models drawn, a switch-off saved with the settings it moves,
// and the reason shown where the models were not read.

import { KitModels } from "@armada/components";
import type { ModelChoices, Setting, SettingsList } from "@armada/protocol";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { provideKitModels, useKitModels } from "./kit-models";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  vi.restoreAllMocks();
});

function Page({ held, settings }: { held: ModelChoices | null; settings: SettingsList | null }) {
  const models = useKitModels(held, settings);
  return <KitModels models={models.models} keep="kit-models-test" picked={undefined} onPick={() => {}} onOn={models.setOn} onChoose={models.choose} />;
}

function draw(held: ModelChoices | null, settings: SettingsList | null) {
  provideKitModels(undefined);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const saved = vi.fn(async () => ({ ok: true as const }));
  (window as unknown as { armada: unknown }).armada = { saveSettings: saved };
  act(() => root.render(<Page held={held} settings={settings} />));
  return saved;
}

const offer = (name: string, display: string, version: string, on = true) => ({ name, display, version, description: `${display} says`, efforts: ["low", "high"], on });
const OFFERED = [offer("opus", "Opus 5.5", "opus-5-5"), offer("sonnet", "Sonnet 5.5", "sonnet-5-5"), offer("haiku", "Haiku 5.5", "haiku-5-5")];
const held = (over: Partial<ModelChoices> = {}): ModelChoices => ({ models: ["opus", "sonnet", "haiku"], default: "sonnet", session: "opus", offered: OFFERED, read: { what: "read" }, ...over });

const setting = (key: string, value: string): Setting => ({
  key, group: "Agents", section: "Model", title: key, description: "", tip: null, kind: { kind: "text" },
  default: value, value, saved: null, applies: "live", pending_restart: false, overridden_by_env: null,
});
const settings = (values: Record<string, string>): SettingsList => ({ path: "/settings.json", refused: null, settings: Object.entries(values).map(([k, v]) => setting(k, v)) });

test("the models Fleet offers are the rows, each with its switch", async () => {
  draw(held(), settings({ "models.review": "opus" }));
  const list = page.getByRole("region", { name: "Models", exact: true });
  for (const name of ["Opus", "Sonnet", "Haiku"]) await expect.element(list.getByRole("button", { name })).toBeVisible();
  await expect.element(page.getByRole("switch", { name: "Haiku may be used" })).toBeChecked();
});

test("switching a model off saves the roster and moves the settings that name it", async () => {
  const saved = draw(held(), settings({ "models.review": "opus", "models.judge": "haiku" }));
  await page.getByRole("switch", { name: "Haiku may be used" }).click({ force: true });
  expect(saved).toHaveBeenCalledWith({ "models.roster": ["opus", "sonnet"], "models.judge": "sonnet" });
});

test("a choice saves the setting its kind of run reads", async () => {
  const saved = draw(held(), settings({ "models.review": "opus" }));
  await page.getByRole("combobox", { name: "Drones" }).selectOptions("haiku");
  expect(saved).toHaveBeenCalledWith({ "models.default": "haiku" });
});

test("the last model on cannot be switched off", async () => {
  draw(held({ offered: [offer("opus", "Opus 5.5", "opus-5-5"), offer("haiku", "Haiku 5.5", "haiku-5-5", false)] }), null);
  await expect.element(page.getByRole("switch", { name: "Opus may be used" })).toBeDisabled();
  await expect.element(page.getByRole("switch", { name: "Haiku may be used" })).toBeEnabled();
});

test("models Fleet could not read show Not read and its reason", async () => {
  draw(held({ read: { what: "not_read", why: "the agent did not answer in time" } }), null);
  await expect.element(page.getByText("Not read", { exact: true })).toBeVisible();
  await expect.element(page.getByText("the agent did not answer in time")).toBeVisible();
});
