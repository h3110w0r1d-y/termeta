import zhCN from "./zh-CN";
import enUS from "./en-US";

export type MessageKey = keyof typeof zhCN;
type MessageSchema = Record<MessageKey, string>;

// 中文资源定义 key 和插值契约；各语言必须实现相同词条。
export const messages = {
  "zh-CN": zhCN,
  "en-US": enUS satisfies MessageSchema,
};

type ParameterNames<Text extends string> =
  Text extends `${string}{${infer Name}}${infer Rest}`
    ? Name | ParameterNames<Rest>
    : never;
type ParametersFor<Key extends MessageKey> = ParameterNames<(typeof zhCN)[Key]>;

export type PlainMessageKey = {
  [Key in MessageKey]: [ParametersFor<Key>] extends [never] ? Key : never;
}[MessageKey];

export type TranslationArguments<Key extends MessageKey> = [
  ParametersFor<Key>,
] extends [never]
  ? [parameters?: never]
  : [parameters: Record<ParametersFor<Key>, string | number>];
