import { t, type PlainMessageKey } from "@/i18n";
import type { FormItemRule, FormInst } from "naive-ui";
export function requiredText(label: PlainMessageKey): FormItemRule {
  return {
    required: true,
    trigger: ["input", "blur", "change"],
    validator: (_, value: unknown) =>
      (typeof value === "string" && !!value.trim()) ||
      new Error(t("common.validation.required", { label: t(label) })),
  };
}
export function integerRange(
  label: PlainMessageKey,
  min: number,
  max: number,
): FormItemRule {
  return {
    required: true,
    type: "number",
    trigger: ["blur", "change"],
    validator: (_, value: unknown) =>
      (typeof value === "number" &&
        Number.isInteger(value) &&
        value >= min &&
        value <= max) ||
      new Error(
        t("common.validation.integerRange", { label: t(label), min, max }),
      ),
  };
}
export async function validateForm(form: FormInst | undefined) {
  if (!form) return false;
  try {
    await form.validate();
    return true;
  } catch {
    return false;
  }
}
