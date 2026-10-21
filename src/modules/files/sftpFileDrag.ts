import type { SftpSelection } from "./types";

export interface SftpDragSelection extends SftpSelection {
  directories: string[];
  nativeAllowed: boolean;
}
export interface DragPoint {
  x: number;
  y: number;
}
export interface SftpDragFeedback extends DragPoint {
  active: boolean;
  count: number;
  overTerminal: boolean;
}
interface DragPorts {
  valid: (selection: SftpDragSelection) => boolean;
  insideWindow: (point: DragPoint) => boolean;
  overTerminal: (point: DragPoint) => boolean;
  update: (feedback: SftpDragFeedback | undefined) => void;
  paste: (selection: SftpDragSelection) => void;
  native: (selection: SftpDragSelection) => void;
}

/** 内部落点和原生拖出只共享手势，不通过系统文件拖放互相转换。 */
export function createSftpFileDrag(ports: DragPorts) {
  let gesture:
    | {
        pointerId: number;
        origin: DragPoint;
        selection: SftpDragSelection;
        active: boolean;
      }
    | undefined;
  function cancel() {
    gesture = undefined;
    ports.update(undefined);
  }
  return {
    cancel,
    start(pointerId: number, origin: DragPoint, selection: SftpDragSelection) {
      cancel();
      if (!selection.names.length || !ports.valid(selection)) return false;
      gesture = { pointerId, origin, selection, active: false };
      ports.update({
        ...origin,
        active: false,
        count: selection.names.length,
        overTerminal: false,
      });
      return true;
    },
    move(pointerId: number, point: DragPoint, buttons: number) {
      if (!gesture || gesture.pointerId !== pointerId) return;
      if (!(buttons & 1) || !ports.valid(gesture.selection)) return cancel();
      if (
        !gesture.active &&
        Math.hypot(point.x - gesture.origin.x, point.y - gesture.origin.y) < 6
      )
        return;
      gesture.active = true;
      if (!ports.insideWindow(point)) {
        const selection = gesture.selection;
        cancel();
        if (selection.nativeAllowed) ports.native(selection);
        return;
      }
      ports.update({
        ...point,
        active: true,
        count: gesture.selection.names.length,
        overTerminal: ports.overTerminal(point),
      });
    },
    end(pointerId: number, point: DragPoint) {
      if (!gesture || gesture.pointerId !== pointerId) return;
      const { selection, active } = gesture;
      const paste =
        active &&
        ports.valid(selection) &&
        ports.insideWindow(point) &&
        ports.overTerminal(point);
      cancel();
      if (paste) ports.paste(selection);
    },
  };
}
