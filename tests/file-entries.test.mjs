import assert from "node:assert/strict";
import test from "node:test";
import {
  directoryRows,
  entryLabel,
  isDirectory,
} from "../src/modules/files/fileEntries.ts";
const entry = (name, kind = "file", targetKind = null) => ({
  name,
  kind,
  targetKind,
  linkTarget: null,
  size: null,
  modified: null,
  permissions: null,
});

test("降序仍保持目录和目录链接优先，上级行置顶且不修改原始分页", () => {
  const files = [
    entry("a"),
    entry("b", "directory"),
    entry("c", "symlink", "directory"),
    entry("z"),
  ];
  const original = structuredClone(files);
  assert.deepEqual(
    directoryRows(files, false, "descend").map((value) => value.name),
    ["..", "c", "b", "z", "a"],
  );
  assert.deepEqual(files, original);
});

test("隐藏开关只过滤点文件，取消排序仍保持组内原序", () => {
  const files = [
    entry("z"),
    entry(".hidden"),
    entry("a"),
    entry("dir", "directory"),
  ];
  assert.deepEqual(
    directoryRows(files, false, false).map((value) => value.name),
    ["..", "dir", "z", "a"],
  );
  assert.deepEqual(
    directoryRows(files, true, false).map((value) => value.name),
    ["..", "dir", "z", ".hidden", "a"],
  );
});

test("断链不作为目录，链接展示保留服务器返回的原始目标", () => {
  const link = { ...entry("link", "symlink"), linkTarget: "../target" };
  assert.equal(isDirectory(link), false);
  assert.equal(entryLabel(link), "link -> ../target");
  assert.equal(entryLabel(entry("plain")), "plain");
});
