// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "json.errors.unterminatedString": "JSON 字符串未结束",
  "json.errors.excessiveDepth": "JSON 嵌套超过 20 层",
  "json.errors.tooManyEntries": "JSON 集合条目过多",
  "json.errors.invalidProperty": "JSON 属性名无效",
  "json.errors.missingColon": "JSON 缺少冒号",
  "json.errors.invalidCollection": "JSON 集合格式错误",
  "json.errors.invalidValue": "JSON 值无效",
  "json.errors.trailingContent": "JSON 含多余内容",
  "json.errors.duplicateProperty": "JSON 存在重复属性：{name}",
} as const;
