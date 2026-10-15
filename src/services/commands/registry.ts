import type { Binding, Modifier } from "../../modules/settings/model";
export const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
export const commandDefinitions = [
  [
    "workspace.newConnection",
    "新建 SSH 连接",
    "application",
    isMac ? "Primary+N" : "Primary+Shift+N",
  ],
  [
    "workspace.closeTab",
    "关闭当前标签",
    "application",
    isMac ? "Primary+W" : "Primary+Shift+W",
  ],
  [
    "workspace.nextTab",
    "下一个标签",
    "application",
    isMac ? "Primary+Shift+]" : "Primary+PageDown",
  ],
  [
    "workspace.previousTab",
    "上一个标签",
    "application",
    isMac ? "Primary+Shift+[" : "Primary+PageUp",
  ],
  ["settings.open", "打开设置", "application", "Primary+,"],
  [
    "terminal.copy",
    "复制选区",
    "terminal",
    isMac ? "Primary+C" : "Primary+Shift+C",
  ],
  [
    "terminal.paste",
    "粘贴",
    "terminal",
    isMac ? "Primary+V" : "Primary+Shift+V",
  ],
  [
    "terminal.find",
    "搜索终端",
    "terminal",
    isMac ? "Primary+F" : "Primary+Shift+F",
  ],
  ["terminal.scrollToBottom", "滚动到底部", "terminal", ""],
  ["terminal.openLink", "打开链接（鼠标主键）", "terminal", ""],
  [
    "workspace.newLocalTerminal",
    "新建本地终端（后续提供）",
    "application",
    isMac ? "Primary+L" : "Primary+Shift+L",
  ],
] as const;
export type CommandId = (typeof commandDefinitions)[number][0];
export function defaults(): Binding[] {
  return commandDefinitions.map(([commandId, , scope, combo]) => {
    const parts = combo.split("+");
    const key = parts.pop() ?? "";
    return {
      id: commandId,
      commandId,
      scope,
      context: scope === "application" ? "workspace" : "terminal",
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
    binding.trigger.type === "key" ? binding.trigger.key : "点击",
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
    if (seen.has(key)) return `快捷键冲突：${seen.get(key)} 与 ${b.commandId}`;
    seen.set(key, b.commandId);
  }
  return "";
}
export function validateBindings(value: unknown): Binding[] {
  if (!Array.isArray(value) || value.length > 100)
    throw new Error("快捷键条目无效");
  const allowed = new Set<string>(commandDefinitions.map((d) => d[0]));
  const modifiers = new Set(["Primary", "Control", "Meta", "Alt", "Shift"]);
  for (const b of value) {
    if (
      !b ||
      !allowed.has(b.commandId) ||
      !["application", "terminal"].includes(b.scope) ||
      !["workspace", "terminal"].includes(b.context) ||
      b.platform !== "all" ||
      typeof b.enabled !== "boolean" ||
      !Array.isArray(b.trigger?.modifiers) ||
      b.trigger.modifiers.some((m: unknown) => !modifiers.has(String(m)))
    )
      throw new Error("快捷键字段不合法");
    const definition = commandDefinitions.find((d) => d[0] === b.commandId)!;
    if (
      b.scope !== definition[2] ||
      b.context !== (b.scope === "application" ? "workspace" : "terminal")
    )
      throw new Error("快捷键上下文与动作不匹配");
    if (b.commandId === "terminal.openLink" && b.trigger.type !== "pointer")
      throw new Error("链接动作需要鼠标绑定");
    if (
      b.trigger.type === "key" &&
      !(b.enabled === false && b.trigger.key === "") &&
      (typeof b.trigger.key !== "string" ||
        b.trigger.key.length > 20 ||
        !/^([\p{L}\p{N}\p{P}\p{S} ]|F(?:[1-9]|1\d|2[0-4])|PageDown|PageUp|ArrowUp|ArrowDown|ArrowLeft|ArrowRight|Enter|Escape|Tab|Backspace|Delete|Home|End)$/u.test(
          b.trigger.key,
        ))
    )
      throw new Error("不支持的按键");
    if (
      b.trigger.type !== "key" &&
      (b.trigger.type !== "pointer" ||
        b.trigger.button !== "primary" ||
        b.trigger.event !== "click" ||
        b.commandId !== "terminal.openLink")
    )
      throw new Error("不支持的触发器");
  }
  const issue = conflict(value);
  if (issue) throw new Error(issue);
  return value;
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
    if (
      event.defaultPrevented ||
      event.isComposing ||
      event.getModifierState("AltGraph") ||
      document.querySelector("[data-modal-open], [data-ui-popup-open]")
    )
      return false;
    const target = event.target as HTMLElement;
    if (
      !terminal &&
      target.closest(
        "input,textarea,select,[role=combobox],[role=slider],[role=spinbutton],[contenteditable=true]",
      )
    )
      return false;
    const candidates = bindings.filter(
      (b) =>
        b.enabled &&
        b.trigger.type === "key" &&
        (b.scope === "application" || terminal),
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
        modifierMatches(event, b.trigger.modifiers) &&
        this.execute(b.commandId)
      ) {
        event.preventDefault();
        event.stopPropagation();
        return true;
      }
    return false;
  }
}
export const commands = new CommandRegistry();
