import { shallowRef } from "vue";
interface DialogRequest {
  message: string;
  input?: string;
  resolve: (value: string | null) => void;
}
export const currentConfirmation = shallowRef<DialogRequest>();
const queue: DialogRequest[] = [];
function request(message: string, input?: string): Promise<string | null> {
  return new Promise((resolve) => {
    queue.push({ message, input, resolve });
    if (!currentConfirmation.value) currentConfirmation.value = queue.shift();
  });
}
export async function confirmAction(message: string) {
  return (await request(message)) !== null;
}
export function promptText(message: string, initial = "") {
  return request(message, initial);
}
export function settleConfirmation(accepted: boolean) {
  const active = currentConfirmation.value;
  if (!active) return;
  currentConfirmation.value = queue.shift();
  active.resolve(accepted ? (active.input ?? "") : null);
}
