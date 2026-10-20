// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "json.errors.unterminatedString": "The JSON text is incomplete.",
  "json.errors.excessiveDepth": "The JSON file has too many nested levels.",
  "json.errors.tooManyEntries": "The JSON file contains too many entries.",
  "json.errors.invalidProperty":
    "The JSON file contains an invalid field name.",
  "json.errors.missingColon": "The JSON file is not valid.",
  "json.errors.invalidCollection": "The JSON file is not valid.",
  "json.errors.invalidValue": "The JSON file contains an invalid value.",
  "json.errors.trailingContent": "The JSON file contains unexpected content.",
  "json.errors.duplicateProperty":
    "The JSON file contains a duplicate field: {name}",
} as const;
