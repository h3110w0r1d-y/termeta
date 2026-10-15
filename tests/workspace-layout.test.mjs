import assert from "node:assert/strict";
import test from "node:test";
import {
  edgeAtPoint,
  layoutGeometry,
  removeTerminal,
  relocateTerminal,
  setSplitRatio,
  splitTerminal,
  terminalIds,
} from "../src/modules/workspace/layout.ts";

const terminal = (terminalId) => ({
  kind: "terminal",
  terminalId,
});

function nestedLayout() {
  const pair = splitTerminal(terminal("a"), "a", terminal("b"), "right");
  return splitTerminal(pair, "b", terminal("c"), "bottom");
}

test("四向分割保持预期顺序、轴与均分比例", () => {
  const cases = [
    ["left", ["b", "a"], "horizontal"],
    ["right", ["a", "b"], "horizontal"],
    ["top", ["b", "a"], "vertical"],
    ["bottom", ["a", "b"], "vertical"],
  ];
  for (const [edge, ids, axis] of cases) {
    const original = terminal("a");
    const result = splitTerminal(original, "a", terminal("b"), edge);
    assert.equal(result.kind, "split");
    if (result.kind !== "split") throw new Error("应创建分屏节点");
    assert.equal(result.axis, axis);
    assert.equal(result.ratio, 0.5);
    assert.ok(result.id);
    assert.deepEqual(terminalIds(result), ids);
    assert.deepEqual(original, terminal("a"));
  }
});

test("嵌套分割覆盖整个舞台且各叶唯一、不重叠、不越界", () => {
  const root = nestedLayout();
  assert.equal(root.kind, "split");
  if (root.kind !== "split") throw new Error("应为分屏布局");
  const resized = setSplitRatio(root, root.id, 0.3);
  const { panes, dividers } = layoutGeometry(resized);
  const ids = terminalIds(resized);
  assert.deepEqual(ids, ["a", "b", "c"]);
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(panes.size, ids.length);
  assert.equal(dividers.length, 2);
  assert.deepEqual(panes.get("a"), {
    left: 0,
    top: 0,
    width: 30,
    height: 100,
  });
  assert.deepEqual(panes.get("c"), {
    left: 30,
    top: 50,
    width: 70,
    height: 50,
  });
  const rectangles = [...panes.values()];
  const area = rectangles.reduce(
    (sum, pane) => sum + pane.width * pane.height,
    0,
  );
  assert.ok(Math.abs(area - 10_000) < 1e-8);
  for (const pane of rectangles) {
    assert.ok(pane.left >= 0 && pane.top >= 0);
    assert.ok(pane.width > 0 && pane.height > 0);
    assert.ok(pane.left + pane.width <= 100 + 1e-8);
    assert.ok(pane.top + pane.height <= 100 + 1e-8);
  }
  for (let i = 0; i < rectangles.length; i++) {
    for (let j = i + 1; j < rectangles.length; j++) {
      const a = rectangles[i];
      const b = rectangles[j];
      const overlapWidth =
        Math.min(a.left + a.width, b.left + b.width) - Math.max(a.left, b.left);
      const overlapHeight =
        Math.min(a.top + a.height, b.top + b.height) - Math.max(a.top, b.top);
      assert.ok(overlapWidth <= 1e-8 || overlapHeight <= 1e-8);
    }
  }
  // 内层分隔线的位置是舞台绝对坐标，bounds 保留内层自身区域。
  assert.deepEqual(dividers[1]?.bounds, {
    left: 30,
    top: 0,
    width: 70,
    height: 100,
  });
  assert.equal(dividers[0]?.position, 30);
  assert.equal(dividers[1]?.position, 50);
});

test("移除叶子折叠空枝，保留其他终端且不改变原树", () => {
  const root = nestedLayout();
  const original = structuredClone(root);
  const result = removeTerminal(root, "b");
  assert.ok(result);
  assert.deepEqual(terminalIds(result), ["a", "c"]);
  assert.deepEqual(root, original);
  if (result.kind !== "split") throw new Error("剩余两个终端应保持分屏");
  assert.deepEqual(result.second, terminal("c"));
  assert.deepEqual(removeTerminal(result, "a"), terminal("c"));
  assert.equal(removeTerminal(terminal("last"), "last"), undefined);
});

test("分割比例限制上下界，NaN 回退均分，递归修改不影响原树", () => {
  const root = nestedLayout();
  const original = structuredClone(root);
  if (root.kind !== "split" || root.second.kind !== "split")
    throw new Error("应包含嵌套分屏");
  const nestedId = root.second.id;
  const cases = [
    [-1, 0.1],
    [2, 0.9],
    [Number.NaN, 0.5],
    [0.37, 0.37],
  ];
  for (const [input, expected] of cases) {
    const result = setSplitRatio(root, nestedId, input);
    if (result.kind !== "split" || result.second.kind !== "split")
      throw new Error("修改比例不得改变树结构");
    assert.equal(result.second.ratio, expected);
    assert.equal(result.first, root.first);
  }
  assert.deepEqual(root, original);
});

test("不存在的目标保持原树引用", () => {
  const root = nestedLayout();
  assert.equal(removeTerminal(root, "missing"), root);
  assert.equal(splitTerminal(root, "missing", terminal("new"), "left"), root);
  assert.equal(setSplitRatio(root, "missing", 0.2), root);
});

test("拖入方向按归一化最近边计算，处理边界与无效尺寸", () => {
  assert.equal(edgeAtPoint(1, 50, 200, 100), "left");
  assert.equal(edgeAtPoint(199, 50, 200, 100), "right");
  assert.equal(edgeAtPoint(100, 1, 200, 100), "top");
  assert.equal(edgeAtPoint(100, 99, 200, 100), "bottom");
  assert.equal(edgeAtPoint(-100, 50, 200, 100), "left");
  assert.equal(edgeAtPoint(100, 200, 200, 100), "bottom");
  assert.equal(edgeAtPoint(0, 0, 0, 0), "left");
  assert.equal(edgeAtPoint(Number.NaN, Number.NaN, 0, -1), "left");
  // 宽终端也应按比例判断，避免以绝对像素距离偏向上下边。
  assert.equal(edgeAtPoint(50, 40, 1000, 100), "left");
});

test("同组窗格重排先收缩再分割，每个终端只出现一次且来源不变", () => {
  for (const edge of ["left", "right", "top", "bottom"]) {
    const root = nestedLayout();
    const snapshot = structuredClone(root);
    const moved = relocateTerminal(root, root, "b", "a", edge);
    assert.ok(moved);
    assert.strictEqual(moved.sourceRoot, moved.targetRoot);
    assert.deepEqual(terminalIds(moved.targetRoot).sort(), ["a", "b", "c"]);
    const panes = layoutGeometry(moved.targetRoot).panes;
    assert.equal(panes.size, 3);
    const a = panes.get("a"),
      b = panes.get("b");
    if (edge === "left") assert.ok(b.left < a.left);
    if (edge === "right") assert.ok(b.left > a.left);
    if (edge === "top") assert.ok(b.top < a.top);
    if (edge === "bottom") assert.ok(b.top > a.top);
    assert.deepEqual(root, snapshot);
  }
});

test("跨组移动收缩来源，目标只新增来源窗格", () => {
  const source = nestedLayout();
  const target = terminal("d");
  const moved = relocateTerminal(source, target, "b", "d", "left");
  assert.ok(moved);
  assert.deepEqual(terminalIds(moved.sourceRoot), ["a", "c"]);
  assert.deepEqual(terminalIds(moved.targetRoot), ["b", "d"]);
  assert.deepEqual(terminalIds(source), ["a", "b", "c"]);
  assert.deepEqual(target, terminal("d"));
});

test("移走仅有叶子清除来源树，无效/自身/重复目标保持原树", () => {
  const source = terminal("a"),
    target = terminal("b");
  const moved = relocateTerminal(source, target, "a", "b", "top");
  assert.ok(moved);
  assert.equal(moved.sourceRoot, undefined);
  assert.deepEqual(terminalIds(moved.targetRoot), ["a", "b"]);
  const pair = splitTerminal(source, "a", target, "right");
  for (const [from, to, id, destination] of [
    [pair, pair, "a", "a"],
    [pair, pair, "missing", "b"],
    [pair, pair, "a", "missing"],
    [source, pair, "a", "b"],
  ])
    assert.equal(
      relocateTerminal(from, to, id, destination, "left"),
      undefined,
    );
  assert.deepEqual(terminalIds(pair), ["a", "b"]);
});
