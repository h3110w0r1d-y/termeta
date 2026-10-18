import { onBeforeUnmount, watch, type Ref } from "vue";

/** 只将顶部标签的纵向滚轮转换为横向位移；渐隐与边缘测量由ScrollFade共用。 */
export function useTabScroll(
  viewport: Ref<HTMLElement | undefined>,
  position: () => "top" | "left",
) {
  let frame = 0;
  let wheelDelta = 0;
  function update() {
    frame = 0;
    if (position() === "top" && viewport.value)
      viewport.value.scrollLeft += wheelDelta;
    wheelDelta = 0;
  }
  function wheel(event: WheelEvent) {
    const current = viewport.value;
    if (
      !current ||
      position() !== "top" ||
      event.defaultPrevented ||
      !event.cancelable ||
      event.ctrlKey ||
      event.metaKey ||
      current.scrollWidth - current.clientWidth <= 1 ||
      !Number.isFinite(event.deltaY) ||
      Math.abs(event.deltaX) >= Math.abs(event.deltaY)
    )
      return;
    // 像素、行与页三种滚轮单位分别归一；原生横向手势由浏览器继续处理。
    const unit =
      event.deltaMode === 1
        ? 32
        : event.deltaMode === 2
          ? current.clientWidth
          : 1;
    event.preventDefault();
    wheelDelta += event.deltaY * unit;
    if (!frame) frame = requestAnimationFrame(update);
  }
  watch(
    viewport,
    (element, _, onCleanup) => {
      element?.addEventListener("wheel", wheel, { passive: false });
      onCleanup(() => element?.removeEventListener("wheel", wheel));
    },
    { immediate: true, flush: "post" },
  );
  // 切换布局时丢弃旧方向的位移，保留同一方向当前帧内合并的滚轮输入。
  watch(
    position,
    () => {
      wheelDelta = 0;
    },
    { flush: "post" },
  );
  onBeforeUnmount(() => {
    if (frame) cancelAnimationFrame(frame);
  });
}
