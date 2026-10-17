import assert from "node:assert/strict";
import test from "node:test";
import {
  moveTabBefore,
  tabInsertionAt,
} from "../src/modules/workbench/tabInsertion.ts";

const tabs = [
  { id: "a", start: 0, end: 100 },
  { id: "b", start: 104, end: 204 },
  { id: "c", start: 208, end: 308 },
  { id: "d", start: 312, end: 412 },
];

test("相邻标签两半与整个4px间隙始终指向同一个插入槽", () => {
  for (const point of [154, 180, 203.9, 204, 205, 206, 207, 208, 230, 257.9]) {
    const slot = tabInsertionAt(tabs, point, "a");
    assert.deepEqual(slot, { beforeGroupId: "c", position: 206 });
    assert.deepEqual(
      moveTabBefore(tabs, "a", slot.beforeGroupId).map((tab) => tab.id),
      ["b", "a", "c", "d"],
    );
  }
  assert.deepEqual(tabInsertionAt(tabs, 258, "a"), {
    beforeGroupId: "d",
    position: 310,
  });
});

test("横纵布局使用同一主轴算法，不依赖固定标签宽度或间距", () => {
  const vertical = [
    { id: "a", start: 10, end: 46 },
    { id: "b", start: 50, end: 86 },
    { id: "c", start: 90, end: 126 },
  ];
  for (const point of [68, 85, 86, 88, 90, 107])
    assert.deepEqual(tabInsertionAt(vertical, point, "a"), {
      beforeGroupId: "c",
      position: 88,
    });
  const unequal = [
    { id: "a", start: 12.5, end: 74.5 },
    { id: "b", start: 81, end: 281 },
    { id: "c", start: 292, end: 334 },
  ];
  assert.deepEqual(tabInsertionAt(unequal, 285, "a"), {
    beforeGroupId: "c",
    position: 286.5,
  });
});

test("滚动后的几何平移不改变插入目标，指示位置跟随实际边界", () => {
  for (const offset of [-300.5, -100, 22.5]) {
    const scrolled = tabs.map((tab) => ({
      ...tab,
      start: tab.start + offset,
      end: tab.end + offset,
    }));
    assert.deepEqual(tabInsertionAt(scrolled, 206 + offset, "a"), {
      beforeGroupId: "c",
      position: 206 + offset,
    });
  }
});

test("源标签前后两个物理槽都属于原位，不产生移动提示", () => {
  for (const point of [50, 102, 120, 154, 206, 250])
    assert.equal(tabInsertionAt(tabs, point, "b"), undefined);
  assert.equal(tabInsertionAt([tabs[0]], 0, "a"), undefined);
  assert.equal(tabInsertionAt([tabs[0]], Infinity, "a"), undefined);
  assert.equal(tabInsertionAt([], 0), undefined);
  assert.equal(tabInsertionAt(tabs, 206, "missing"), undefined);
});

test("首尾槽与窗格插入不受源标签原位过滤影响", () => {
  assert.deepEqual(tabInsertionAt(tabs, -1, "d"), {
    beforeGroupId: "a",
    position: 0,
  });
  for (const point of [362, 412, 1000, Infinity])
    assert.deepEqual(tabInsertionAt(tabs, point, "a"), {
      beforeGroupId: null,
      position: 412,
    });
  assert.deepEqual(tabInsertionAt(tabs, 102), {
    beforeGroupId: "b",
    position: 102,
  });
  assert.deepEqual(tabInsertionAt(tabs, 206), {
    beforeGroupId: "c",
    position: 206,
  });
});

test("双向与首尾排序先移除来源，保留标签对象和原数组", () => {
  const cases = [
    ["a", "c", ["b", "a", "c", "d"]],
    ["a", null, ["b", "c", "d", "a"]],
    ["d", "b", ["a", "d", "b", "c"]],
    ["d", "a", ["d", "a", "b", "c"]],
    ["b", "d", ["a", "c", "b", "d"]],
  ];
  const snapshot = structuredClone(tabs);
  for (const [source, before, expected] of cases) {
    const result = moveTabBefore(tabs, source, before);
    assert.deepEqual(
      result.map((tab) => tab.id),
      expected,
    );
    assert.equal(new Set(result).size, tabs.length);
    for (const tab of result)
      assert.strictEqual(
        tab,
        tabs.find((original) => original.id === tab.id),
      );
    assert.deepEqual(tabs, snapshot);
  }
});

test("原位或失效目标不替换数组，避免无效排序触发响应式更新", () => {
  for (const [source, before] of [
    ["b", "c"],
    ["d", null],
    ["b", "b"],
    ["missing", "a"],
    ["a", "missing"],
  ])
    assert.strictEqual(moveTabBefore(tabs, source, before), tabs);
});
