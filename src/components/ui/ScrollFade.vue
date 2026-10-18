<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    viewport?: HTMLElement | null;
    content?: HTMLElement | null;
    direction?: "horizontal" | "vertical";
    background?: string;
    size?: number;
  }>(),
  {
    direction: "vertical",
    background: "var(--bg)",
    size: 20,
  },
);
const root = ref<HTMLElement>();
// NScrollbar公开实例仅提供滚动方法，DOM适配集中在此处，与现有拖动命中约定一致。
const viewportElement = computed(
  () =>
    props.viewport ??
    root.value?.querySelector<HTMLElement>(".n-scrollbar-container"),
);
const contentElement = computed(
  () =>
    props.content ??
    viewportElement.value?.querySelector<HTMLElement>(".n-scrollbar-content"),
);
const edges = reactive({ start: false, end: false });
const style = computed(() => ({
  "--scroll-fade-background": props.background,
  "--scroll-fade-size": `${Number.isFinite(props.size) ? Math.max(0, props.size) : 20}px`,
}));
let frame = 0;
let disposed = false;
function update() {
  frame = 0;
  const element = viewportElement.value;
  const vertical = props.direction === "vertical";
  const page = element
    ? vertical
      ? element.clientHeight
      : element.clientWidth
    : 0;
  if (!element || page <= 0) {
    edges.start = edges.end = false;
    return;
  }
  const extent = Math.max(
    0,
    (vertical ? element.scrollHeight : element.scrollWidth) - page,
  );
  // 容忍WebKit弹性滚动与小数坐标，到边缘后立即移除对应遮罩。
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
watch(
  () => [viewportElement.value, contentElement.value] as const,
  ([viewport, content], _, onCleanup) => {
    schedule();
    if (!viewport) return;
    const scrollElement = viewport;
    const resize = new ResizeObserver(schedule);
    resize.observe(scrollElement);
    // 库滚动区观察内容节点；原生标签区观察直属项，覆盖字体与标题宽度变化。
    const observed = new Set<Element>();
    function observeContent() {
      const elements = new Set<Element>(
        content ? [content] : [...scrollElement.children],
      );
      for (const element of observed) {
        if (!elements.has(element)) {
          resize.unobserve(element);
          observed.delete(element);
        }
      }
      for (const element of elements) {
        if (!observed.has(element)) {
          resize.observe(element);
          observed.add(element);
        }
      }
    }
    observeContent();
    const mutation = new MutationObserver(() => {
      observeContent();
      schedule();
    });
    mutation.observe(content ?? viewport, {
      childList: true,
      characterData: true,
      subtree: true,
    });
    viewport.addEventListener("scroll", schedule, { passive: true });
    onCleanup(() => {
      resize.disconnect();
      mutation.disconnect();
      viewport.removeEventListener("scroll", schedule);
    });
  },
  { immediate: true, flush: "post" },
);
watch(() => props.direction, schedule, { flush: "post" });
onBeforeUnmount(() => {
  disposed = true;
  if (frame) cancelAnimationFrame(frame);
});
</script>

<template>
  <div
    ref="root"
    class="scroll-fade"
    :class="{
      'scroll-fade-horizontal': direction === 'horizontal',
      'can-scroll-start': edges.start,
      'can-scroll-end': edges.end,
    }"
    :style="style"
  >
    <slot />
  </div>
</template>

<style scoped>
.scroll-fade {
  position: relative;
  min-width: 0;
  min-height: 0;
  isolation: isolate;
  --scroll-fade-direction: to bottom;
}
/* 遮罩只绘制背景色渐隐，不接收指针、不生成可聚焦元素。 */
.scroll-fade::before,
.scroll-fade::after {
  content: "";
  display: none;
  position: absolute;
  z-index: 2;
  pointer-events: none;
  inset-inline: 0;
  height: min(var(--scroll-fade-size), 20%);
  background: linear-gradient(
    var(--scroll-fade-direction),
    var(--scroll-fade-background),
    transparent
  );
}
.scroll-fade::before {
  top: 0;
}
.scroll-fade::after {
  bottom: 0;
  transform: rotate(180deg);
}
.scroll-fade.can-scroll-start::before,
.scroll-fade.can-scroll-end::after {
  display: block;
}
.scroll-fade-horizontal {
  --scroll-fade-direction: to right;
}
.scroll-fade-horizontal::before,
.scroll-fade-horizontal::after {
  inset-block: 0;
  height: auto;
  width: min(var(--scroll-fade-size), 20%);
}
.scroll-fade-horizontal::before {
  left: -1px;
  right: auto;
}
.scroll-fade-horizontal::after {
  left: auto;
  right: 0;
}
</style>
