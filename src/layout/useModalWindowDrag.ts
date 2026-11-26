import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { useWindowDrag } from "@/components/useWindowDrag";

/** 遮罩覆盖 Header 时，将整条区域的空白手势交给窗口，保留模态交互边界。 */
export function useModalWindowDrag(
  header: Ref<HTMLElement | undefined>,
  onError: (message: string) => void,
) {
  let dragMask: Element | undefined;
  const windowDrag = useWindowDrag({
    enabled: () => !!dragMask?.isConnected && !!header.value?.isConnected,
    onError,
  });

  function pointerDown(event: PointerEvent) {
    if (
      event.button !== 0 ||
      !event.isPrimary ||
      event.pointerType === "touch" ||
      !(event.target instanceof Element)
    )
      return;
    const bounds = header.value?.getBoundingClientRect();
    if (
      !bounds ||
      event.clientX < bounds.left ||
      event.clientX >= bounds.right ||
      event.clientY < bounds.top ||
      event.clientY >= bounds.bottom
    )
      return;

    // Naive UI 的空白处可能命中滚动包装；实际弹窗与传送到外部的弹层保留交互。
    const container = event.target.closest(".n-modal-container");
    const mask = container?.querySelector(".n-modal-mask");
    if (!mask || event.target.closest(".n-modal")) return;

    dragMask = mask;
    // 阻止遮罩夺走表单焦点及生成兼容鼠标事件，不向底层 Header 转发事件。
    event.preventDefault();
    event.stopPropagation();
    windowDrag.start(event);
  }

  onMounted(() => document.addEventListener("pointerdown", pointerDown, true));
  onBeforeUnmount(() => {
    document.removeEventListener("pointerdown", pointerDown, true);
    dragMask = undefined;
  });
}
