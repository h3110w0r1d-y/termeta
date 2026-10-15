import { shallowRef } from "vue";
interface PasteRequest {
  text: string;
  lines: number;
  targetCount: number;
  settle: (accepted: boolean) => void;
}
export const pendingPaste = shallowRef<PasteRequest>();
export function confirmPaste(text: string, targetCount = 1): Promise<boolean> {
  if (pendingPaste.value) return Promise.resolve(false);
  return new Promise((resolve) => {
    pendingPaste.value = {
      text,
      targetCount,
      lines: text.split(/\r\n|\r|\n/).length,
      settle: resolve,
    };
  });
}
export function resolvePaste(accepted: boolean) {
  const request = pendingPaste.value;
  pendingPaste.value = undefined;
  request?.settle(accepted);
}
