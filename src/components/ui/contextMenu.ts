import { isMac } from "@/services/commands/registry";

export function closeContextMenuOnOutside(
  event: MouseEvent,
  menu: { show: boolean },
) {
  // Naive UI的clickoutside由mousedown/mouseup合成，也包含右键。
  // 菜单保活后，同一次右键按下打开的菜单会在松开时被误关；
  // 右键交给contextmenu定位/切换目标，外部关闭忽略它及macOS的Control+单击。
  if (event.button === 2 || (isMac && event.button === 0 && event.ctrlKey))
    return;
  menu.show = false;
}
