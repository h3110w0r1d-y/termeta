import { ref } from "vue";
interface DialogRequest {
  message: string;
  input?: string;
  resolve: (value: string | null) => void;
}
// 输入框直接绑定请求的input字段，需要跟踪嵌套更新以同步受控组件的值。
export const currentConfirmation = ref<DialogRequest>();
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
