import { t } from "../../i18n";
// Pointer Events 不提供原生拖动影像；使用只读DOM快照保留库组件的当前外观。
export function createDragPreview(
  source: HTMLElement,
  originX: number,
  originY: number,
  preview: HTMLElement,
) {
  const rect = source.getBoundingClientRect();
  const offsetX = originX - rect.left;
  const offsetY = originY - rect.top;
  preview.style.width = `${rect.width}px`;
  preview.style.display = "block";
  const copy = source.cloneNode(true) as HTMLElement;
  const originals = [source, ...source.querySelectorAll("*")];
  const copies = [copy, ...copy.querySelectorAll("*")];
  // 快照计算样式，避免移到body后丢失树行的祖先选择器、主题变量与继承字体。
  originals.forEach((original, index) => {
    const cloned = copies[index];
    if (!(cloned instanceof HTMLElement || cloned instanceof SVGElement))
      return;
    const computed = getComputedStyle(original);
    for (const property of computed) {
      cloned.style.setProperty(property, computed.getPropertyValue(property));
    }
    cloned.removeAttribute("id");
    cloned.removeAttribute("autofocus");
    cloned.removeAttribute("data-host-group-target");
    cloned.removeAttribute("data-ui-popup-open");
    cloned.setAttribute("tabindex", "-1");
    cloned.setAttribute("draggable", "false");
    // 子元素同样穿透指针，确保elementFromPoint命中真实目标而非快照。
    cloned.style.setProperty("pointer-events", "none", "important");
    cloned.style.setProperty("transition", "none", "important");
    cloned.style.setProperty("animation", "none", "important");
  });
  Object.assign(copy.style, {
    position: "relative",
    inset: "auto",
    margin: "0",
    boxSizing: "border-box",
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    opacity: "0.65",
  });
  const hint = document.createElement("div");
  hint.className = "host-drag-hint";
  preview.append(copy, hint);
  const opacity = source.style.getPropertyValue("opacity");
  const priority = source.style.getPropertyPriority("opacity");
  source.style.setProperty("opacity", "0.4");
  return {
    move(x: number, y: number, target: string | null) {
      preview.style.transform = `translate3d(${x - offsetX}px, ${y - offsetY}px, 0)`;
      hint.textContent = target
        ? t("hosts.drag.destination", { target: target })
        : t("hosts.drag.invalidTarget");
    },
    dispose() {
      preview.replaceChildren();
      preview.style.display = "none";
      if (opacity) source.style.setProperty("opacity", opacity, priority);
      else source.style.removeProperty("opacity");
    },
  };
}
