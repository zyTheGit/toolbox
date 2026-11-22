import assert from "node:assert/strict";
import test from "node:test";
import {
  acceleratorFromCombo,
  canonicalAccelerator,
  formatModifierPreview,
  formatShortcut,
  hasModifier,
  isModifierCode,
  matchesShortcut,
  normalizePlatform,
} from "../src/utils/shortcut.ts";

function combo(
  code: string,
  mods: Partial<{ ctrl: boolean; alt: boolean; shift: boolean; meta: boolean }> = {},
) {
  return {
    code,
    ctrlKey: mods.ctrl ?? false,
    altKey: mods.alt ?? false,
    shiftKey: mods.shift ?? false,
    metaKey: mods.meta ?? false,
  };
}

test("acceleratorFromCombo builds a parseable accelerator on Windows", () => {
  const value = acceleratorFromCombo(combo("KeyT", { ctrl: true, shift: true }), "windows");
  assert.equal(value, "CommandOrControl+Shift+KeyT");
});

test("acceleratorFromCombo maps Cmd to the primary modifier on macOS", () => {
  const value = acceleratorFromCombo(combo("KeyK", { meta: true, alt: true }), "macos");
  assert.equal(value, "CommandOrControl+Alt+KeyK");
});

test("acceleratorFromCombo keeps the Windows key distinct from Ctrl", () => {
  const value = acceleratorFromCombo(combo("KeyL", { meta: true }), "windows");
  assert.equal(value, "Super+KeyL");
});

test("acceleratorFromCombo rejects bare keys and unsupported keys", () => {
  assert.equal(acceleratorFromCombo(combo("KeyA"), "windows"), null);
  assert.equal(acceleratorFromCombo(combo("IntlBackslash", { ctrl: true }), "windows"), null);
});

test("acceleratorFromCombo accepts function and navigation keys with modifiers", () => {
  assert.equal(acceleratorFromCombo(combo("F8", { ctrl: true }), "windows"), "CommandOrControl+F8");
  assert.equal(acceleratorFromCombo(combo("ArrowUp", { alt: true }), "windows"), "Alt+ArrowUp");
});

test("formatShortcut renders existing config values for each platform", () => {
  assert.equal(formatShortcut("CommandOrControl+Shift+T", "windows"), "Ctrl + Shift + T");
  assert.equal(formatShortcut("CommandOrControl+Shift+T", "macos"), "Cmd + Shift + T");
  assert.equal(formatShortcut("CommandOrControl+ArrowUp", "windows"), "Ctrl + ↑");
  assert.equal(formatShortcut("Ctrl+NumpadAdd", "windows"), "Ctrl + Num+");
});

test("formatModifierPreview shows the modifiers held during recording", () => {
  assert.equal(formatModifierPreview(combo("ShiftLeft", { ctrl: true, shift: true }), "windows"), "Ctrl + Shift");
  assert.equal(formatModifierPreview(combo("MetaLeft", { meta: true }), "macos"), "Cmd");
});

test("modifier helpers recognize physical modifier codes and platform aliases", () => {
  assert.equal(isModifierCode("ControlLeft"), true);
  assert.equal(isModifierCode("KeyA"), false);
  assert.equal(hasModifier(combo("KeyA", { shift: true })), true);
  assert.equal(normalizePlatform("darwin"), "macos");
  assert.equal(normalizePlatform(""), "windows");
});

test("canonicalAccelerator folds aliases and modifier order", () => {
  assert.equal(canonicalAccelerator("Shift+Ctrl+K", "windows"), "control+shift+KeyK");
  assert.equal(canonicalAccelerator("Ctrl+Enter", "windows"), "control+Enter");
  assert.equal(canonicalAccelerator("CommandOrControl+Enter", "macos"), "super+Enter");
  assert.equal(canonicalAccelerator("Control+Return", "linux"), "control+Enter");
  assert.equal(canonicalAccelerator("Shift+K", "windows"), "shift+KeyK");
  assert.equal(canonicalAccelerator("Ctrl+Shift", "windows"), null);
  assert.equal(canonicalAccelerator("", "windows"), null);
});

test("matchesShortcut compares recorded combos against configured bindings", () => {
  assert.equal(matchesShortcut(combo("Enter", { ctrl: true }), "CommandOrControl+Enter", "windows"), true);
  assert.equal(matchesShortcut(combo("Enter", { meta: true }), "CommandOrControl+Enter", "macos"), true);
  assert.equal(matchesShortcut(combo("Enter", { meta: true }), "CommandOrControl+Enter", "windows"), false);
  assert.equal(matchesShortcut(combo("Enter", { ctrl: true, shift: true }), "CommandOrControl+Enter", "windows"), false);
  assert.equal(matchesShortcut(combo("Enter"), "CommandOrControl+Enter", "windows"), false);
  assert.equal(matchesShortcut(combo("KeyK", { ctrl: true }), "Ctrl+K", "windows"), true);
  assert.equal(matchesShortcut(combo("KeyK", { ctrl: true }), "  ", "windows"), false);
});
