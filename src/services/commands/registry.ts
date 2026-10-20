import { t } from "@/i18n";
import type { Binding, Keymap, Modifier } from "./types";
export const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
export const commandDefinitions = [
  ["workbench.newTab", "workbench.actions.newTab", "application", "Primary+T"],
  [
    "workbench.newConnection",
    "workbench.actions.newSshConnection",
    "application",
    isMac ? "Primary+N" : "Primary+Shift+N",
  ],
  [
    "workbench.closeTab",
    "workbench.actions.closeCurrentTab",
    "application",
    isMac ? "Primary+W" : "Primary+Shift+W",
  ],
  [
    "workbench.nextTab",
    "workbench.actions.nextTab",
    "application",
    isMac ? "Primary+Shift+]" : "Primary+PageDown",
  ],
  [
    "workbench.previousTab",
    "workbench.actions.previousTab",
    "application",
    isMac ? "Primary+Shift+[" : "Primary+PageUp",
  ],
  [
    "settings.open",
    "settings.actions.open",
    "application",
    isMac ? "" : "Primary+,",
  ],
  [
    "terminal.copy",
    "terminal.actions.copySelection",
    "terminal",
    isMac ? "Primary+C" : "Primary+Shift+C",
  ],
  [
    "terminal.paste",
    "terminal.actions.paste",
    "terminal",
    isMac ? "Primary+V" : "Primary+Shift+V",
  ],
  [
    "terminal.find",
    "terminal.actions.find",
    "terminal",
    isMac ? "Primary+F" : "Primary+Shift+F",
  ],
  [
    "terminal.scrollToBottom",
    "terminal.actions.scrollToBottom",
    "terminal",
    "",
  ],
  ["terminal.openLink", "terminal.actions.openLink", "terminal", ""],
  [
    "workbench.newLocalTerminal",
    "workbench.actions.newLocalTerminal",
    "application",
    isMac ? "Primary+L" : "Primary+Shift+L",
  ],
] as const;
export type CommandId = (typeof commandDefinitions)[number][0];
export function commandLabel(commandId: string): string {
  const definition = commandDefinitions.find((item) => item[0] === commandId);
  return definition ? t(definition[1]) : commandId;
}
export function defaults(): Binding[] {
  return commandDefinitions.map(([commandId, , scope, combo]) => {
    const parts = combo.split("+");
    const key = parts.pop() ?? "";
    return {
      id: commandId,
      commandId,
      scope,
      context: scope === "application" ? "workbench" : "terminal",
      trigger:
        commandId === "terminal.openLink"
          ? {
              type: "pointer",
              button: "primary",
              event: "click",
              modifiers: ["Primary"],
            }
          : { type: "key", key, modifiers: parts as Modifier[] },
      platform: "all",
      enabled: !!combo || commandId === "terminal.openLink",
    };
  });
}
export function modifierMatches(
  event: MouseEvent | KeyboardEvent,
  modifiers: Modifier[],
) {
  const has = (key: Modifier) =>
    modifiers.includes(key) ||
    (modifiers.includes("Primary") && key === (isMac ? "Meta" : "Control"));
  return (
    event.ctrlKey === has("Control") &&
    event.metaKey === has("Meta") &&
    event.altKey === has("Alt") &&
    event.shiftKey === has("Shift")
  );
}
export function keyLabel(binding: Binding) {
  return [
    ...binding.trigger.modifiers.map((m) =>
      m === "Primary" ? (isMac ? "Cmd" : "Ctrl") : m,
    ),
    binding.trigger.type === "key"
      ? binding.trigger.key
      : t("shortcuts.triggers.click"),
  ].join("+");
}
export function conflict(bindings: Binding[]) {
  const seen = new Map<string, string>();
  for (const b of bindings.filter((b) => b.enabled)) {
    const mods = b.trigger.modifiers
      .map((m) => (m === "Primary" ? (isMac ? "Meta" : "Control") : m))
      .sort()
      .join("+");
    const key = `${b.scope}/${b.trigger.type}/${mods}/${b.trigger.type === "key" ? b.trigger.key.toLowerCase() : "click"}`;
    if (seen.has(key))
      return t("shortcuts.validation.conflict", {
        first: commandLabel(seen.get(key)!),
        second: commandLabel(b.commandId),
      });
    seen.set(key, b.commandId);
  }
  return "";
}
export function validateBindings(value: unknown): Binding[] {
  if (!Array.isArray(value) || value.length > 100)
    throw new Error(t("shortcuts.validation.invalidEntries"));
  const allowed = new Set<string>(commandDefinitions.map((d) => d[0]));
  const modifiers = new Set(["Primary", "Control", "Meta", "Alt", "Shift"]);
  for (const b of value) {
    if (
      !b ||
      !allowed.has(b.commandId) ||
      !["application", "terminal"].includes(b.scope) ||
      !["workbench", "terminal"].includes(b.context) ||
      b.platform !== "all" ||
      typeof b.enabled !== "boolean" ||
      !Array.isArray(b.trigger?.modifiers) ||
      b.trigger.modifiers.some((m: unknown) => !modifiers.has(String(m)))
    )
      throw new Error(t("shortcuts.validation.invalidFields"));
    const definition = commandDefinitions.find((d) => d[0] === b.commandId)!;
    if (
      b.scope !== definition[2] ||
      b.context !== (b.scope === "application" ? "workbench" : "terminal")
    )
      throw new Error(t("shortcuts.validation.contextMismatch"));
    if (b.commandId === "terminal.openLink" && b.trigger.type !== "pointer")
      throw new Error(t("shortcuts.validation.linkRequiresMouse"));
    if (
      b.trigger.type === "key" &&
      !(b.enabled === false && b.trigger.key === "") &&
      (typeof b.trigger.key !== "string" ||
        b.trigger.key.length > 20 ||
        !/^([\p{L}\p{N}\p{P}\p{S} ]|F(?:[1-9]|1\d|2[0-4])|PageDown|PageUp|ArrowUp|ArrowDown|ArrowLeft|ArrowRight|Enter|Escape|Tab|Backspace|Delete|Home|End)$/u.test(
          b.trigger.key,
        ))
    )
      throw new Error(t("shortcuts.validation.unsupportedKey"));
    if (
      b.trigger.type !== "key" &&
      (b.trigger.type !== "pointer" ||
        b.trigger.button !== "primary" ||
        b.trigger.event !== "click" ||
        b.commandId !== "terminal.openLink")
    )
      throw new Error(t("shortcuts.validation.unsupportedTrigger"));
  }
  const issue = conflict(value);
  if (issue) throw new Error(issue);
  return value;
}
/** 只接受当前格式；保留导入的绑定集合，不自动补充用户未配置的动作。 */
export function parseKeymap(keymap: unknown): Keymap {
  if (
    !keymap ||
    typeof keymap !== "object" ||
    !("schemaVersion" in keymap) ||
    keymap.schemaVersion !== 2
  )
    throw new Error(t("shortcuts.validation.unsupportedVersion"));
  return {
    schemaVersion: 2,
    bindings: validateBindings(
      "bindings" in keymap ? keymap.bindings : undefined,
    ),
  };
}
export class CommandRegistry {
  private handlers = new Map<string, () => void>();
  register(id: CommandId, handler: () => void) {
    this.handlers.set(id, handler);
  }
  execute(id: string) {
    const handler = this.handlers.get(id);
    if (!handler) return false;
    handler();
    return true;
  }
  route(event: KeyboardEvent, bindings: Binding[], terminal: boolean) {
    // macOS 的固定 Cmd+, 仅由原生菜单处理；已有保存绑定也不参与前端路由。
    // 原生事件监视器在 WebKit 前消费该组合；此处仅防止残余事件重复执行动作。
    if (
      isMac &&
      event.metaKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      event.key === ","
    )
      return false;
    // 弹窗不执行关闭标签动作，也必须消费系统Cmd+W，防止回落到关闭整个窗口。
    // 放在录制/IME等提前返回之前，且独立于用户是否改绑或禁用关闭标签动作。
    const modalOpen = !!document.querySelector("[data-modal-open]");
    if (
      modalOpen &&
      isMac &&
      event.metaKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      (event.key.toLowerCase() === "w" || event.code === "KeyW")
    ) {
      event.preventDefault();
      event.stopPropagation();
      return true;
    }
    if (
      (event.target instanceof Element &&
        event.target.closest("[data-shortcut-recorder]")) ||
      event.defaultPrevented ||
      event.isComposing ||
      event.getModifierState("AltGraph") ||
      modalOpen
    )
      return false;
    // 页面输入框不改变应用动作的作用域；模态、录制和其他弹层仍优先。
    const popupBlocksShortcuts = !!document.querySelector(
      "[data-ui-popup-open]:not([data-application-shortcuts])",
    );
    const candidates = bindings.filter(
      (b) =>
        b.enabled &&
        b.trigger.type === "key" &&
        (b.scope === "application" || terminal) &&
        !popupBlocksShortcuts,
    );
    candidates.sort(
      (a, b) => Number(b.scope === "terminal") - Number(a.scope === "terminal"),
    );
    for (const b of candidates)
      if (
        b.trigger.type === "key" &&
        b.trigger.key.toLowerCase() ===
          (event.shiftKey && event.key === "}"
            ? "]"
            : event.shiftKey && event.key === "{"
              ? "["
              : event.key
          ).toLowerCase() &&
        modifierMatches(event, b.trigger.modifiers)
      ) {
        // 标准粘贴交给系统产生带正文的paste事件，避免WebKit异步读剪贴板弹出授权菜单。
        // 终端宿主捕获paste后仍统一执行大小限制、多行确认和广播代次校验。
        const nativePaste =
          terminal &&
          b.commandId === "terminal.paste" &&
          event.key.toLowerCase() === "v" &&
          !event.altKey &&
          (isMac
            ? event.metaKey && !event.ctrlKey && !event.shiftKey
            : event.ctrlKey && !event.metaKey && event.shiftKey);
        if (nativePaste) return false;
        if (!this.execute(b.commandId)) continue;
        event.preventDefault();
        event.stopPropagation();
        return true;
      }
    return false;
  }
}
export const commands = new CommandRegistry();
