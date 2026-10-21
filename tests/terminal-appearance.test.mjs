import assert from "node:assert/strict";
import test from "node:test";
import { darkTheme, lightTheme } from "naive-ui";
import { colorOverlay, mixColors } from "../src/modules/terminal/appearance.ts";

test("终端表面颜色为具体 RGBA，保留透明通道及混合边界", () => {
  assert.equal(mixColors("#9D9D9D", "#000000", 4), "rgba(6, 6, 6, 1)");
  assert.equal(mixColors("#000000", "#ffffff", 4), "rgba(245, 245, 245, 1)");
  assert.equal(
    colorOverlay("#11223380", 20),
    `rgba(17, 34, 51, ${(128 / 255) * 0.2})`,
  );
  assert.equal(mixColors("#ff000000", "#0000ff", 50), "rgba(0, 0, 255, 0.5)");
  assert.equal(mixColors("#ff000000", "#0000ff00", 50), "rgba(0, 0, 0, 0)");
  assert.equal(mixColors("#123456", "#abcdef", 0), "rgba(171, 205, 239, 1)");
  assert.equal(mixColors("#123456", "#abcdef", 100), "rgba(18, 52, 86, 1)");
});

test("浅色与深色 SFTP 表面可通过 Naive UI 表格的颜色派生", () => {
  for (const [theme, background, ui] of [
    [darkTheme, "#000000", "#9D9D9D"],
    [lightTheme, "#ffffff", "#11223380"],
  ]) {
    const surface = mixColors(ui, background, 4);
    const common = {
      ...theme.common,
      cardColor: surface,
      modalColor: surface,
      popoverColor: surface,
      tableHeaderColor: surface,
      tableColorHover: mixColors(ui, background, 12),
      tableColorStriped: surface,
      dividerColor: colorOverlay(ui, 20),
    };
    const derived = theme.DataTable.self(common);
    assert.ok(derived.thColor);
    assert.ok(derived.borderColor);
    assert.ok(derived.tdColorHover);
  }
});
