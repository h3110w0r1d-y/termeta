// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "json.errors.unterminatedString": "JSONテキストが不完全です。",
  "json.errors.excessiveDepth": "JSONファイルの入れ子が深すぎます。",
  "json.errors.tooManyEntries": "JSONファイルの項目が多すぎます。",
  "json.errors.invalidProperty":
    "JSONファイルに無効なフィールド名が含まれています。",
  "json.errors.missingColon": "JSONファイルの形式が無効です。",
  "json.errors.invalidCollection": "JSONファイルの形式が無効です。",
  "json.errors.invalidValue": "JSONファイルに無効な値が含まれています。",
  "json.errors.trailingContent":
    "JSONファイルに予期しない内容が含まれています。",
  "json.errors.duplicateProperty":
    "JSONファイルに重複したフィールドがあります：{name}",
} as const;
