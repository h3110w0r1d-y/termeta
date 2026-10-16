import { onBeforeUnmount, onMounted, reactive, watch, type Ref } from "vue";

/** 仅管理标签列表的滚轮与边缘提示，不参与终端滚动、选中或拖动状态。 */
export function useTabScroll(
  viewport: Ref<HTMLElement | undefined>,
  position: () => "top" | "left",
  content: () => unknown,
) {
  const edges = reactive({ start: false, end: false });
  let observer: ResizeObserver | undefined;
  let frame = 0;
  let wheelDelta = 0;
  let extent = 0;
  let pageSize = 0;
  let disposed = false;

  function update() {
    frame = 0;
    const element = viewport.value;
    if (!element || disposed) return;
    const vertical = position() === "left";
    if (wheelDelta !== 0) {
      // 一帧只提交一次滚轮位移；不启动反复重置的平滑滚动动画。
      if (!vertical) element.scrollLeft += wheelDelta;
      wheelDelta = 0;
    }
    pageSize = vertical ? element.clientHeight : element.clientWidth;
    extent = Math.max(
      0,
      (vertical ? element.scrollHeight : element.scrollWidth) - pageSize,
    );
    // WebKit弹性滚动和小数坐标不能让已到达的边缘仍保留遮罩。
    const offset = Math.max(
      0,
      Math.min(extent, vertical ? element.scrollTop : element.scrollLeft),
    );
    edges.start = offset > 1;
    edges.end = extent - offset > 1;
  }
  function schedule() {
    if (!frame && !disposed) frame = requestAnimationFrame(update);
  }
  function wheel(event: WheelEvent) {
    if (
      position() !== "top" ||
      event.defaultPrevented ||
      !event.cancelable ||
      event.ctrlKey ||
      event.metaKey ||
      extent <= 1 ||
      !Number.isFinite(event.deltaY) ||
      Math.abs(event.deltaX) >= Math.abs(event.deltaY)
    )
      return;
    // 像素、行与页三种滚轮单位分别归一；原生横向手势由浏览器继续处理。
    const unit =
      event.deltaMode === 1 ? 32 : event.deltaMode === 2 ? pageSize : 1;
    event.preventDefault();
    wheelDelta += event.deltaY * unit;
    schedule();
  }
  onMounted(() => {
    const element = viewport.value;
    if (!element) return;
    element.addEventListener("wheel", wheel, { passive: false });
    element.addEventListener("scroll", schedule, { passive: true });
    observer = new ResizeObserver(schedule);
    observer.observe(element);
    schedule();
  });
  // 切换布局时丢弃旧方向的位移；标题更新不能吞掉当前帧尚未提交的滚轮输入。
  watch(
    position,
    () => {
      wheelDelta = 0;
      schedule();
    },
    { flush: "post" },
  );
  // 标签增删、排序及远端标题变宽后重算；不监听终端字节流。
  watch(content, schedule, { flush: "post" });
  onBeforeUnmount(() => {
    disposed = true;
    if (frame) cancelAnimationFrame(frame);
    observer?.disconnect();
    viewport.value?.removeEventListener("wheel", wheel);
    viewport.value?.removeEventListener("scroll", schedule);
  });
  return { edges };
}
