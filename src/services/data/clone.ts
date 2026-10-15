// DTO 仅含可序列化数据；通过 JSON 脱离 Vue Proxy，避免 structuredClone 拒绝响应式代理。
export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
