// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "workspaces.title": "작업 공간",
  "workspaces.errors.missingLayout":
    "이 작업 공간에 저장된 레이아웃이 없습니다.",
  "workspaces.actions.save": "작업 공간 저장",
  "workspaces.actions.saveMenu": "작업 공간 저장…",
  "workspaces.save.description":
    "현재 레이아웃과 연결을 저장합니다. 같은 이름으로 다시 저장하면 이 작업 공간이 업데이트됩니다.",
  "workspaces.errors.workspaceOnly":
    "여기에서는 작업 공간만 저장할 수 있습니다.",
  "workspaces.errors.nameConflict":
    "연결에서 이미 사용 중인 이름입니다. 같은 이름의 작업 공간만 교체할 수 있습니다.",
  "workspaces.save.confirmReplace":
    "작업 공간 “{name}”의 레이아웃과 연결을 교체합니다.",
  "workspaces.errors.deletedConnection":
    "작업 공간 “{name}”에서 사용하는 연결이 삭제되었습니다. 작업 공간을 다시 저장하세요.",
  "workspaces.errors.singleConnection": "보관함에서 이 작업 공간을 여세요.",
  "workspaces.errors.invalidSplit": "작업 공간 레이아웃이 올바르지 않습니다.",
  "workspaces.errors.unsavedConnections":
    "작업 공간을 저장하기 전에 모든 SSH, Telnet 또는 시리얼 연결을 저장하세요. 로컬 터미널은 아직 저장할 수 없습니다.",
  "workspaces.errors.layoutLimit":
    "작업 공간 레이아웃이 지원하는 크기를 초과했습니다.",
  "workspaces.errors.invalidFocus": "선택한 작업 공간 창이 올바르지 않습니다.",
  "workspaces.errors.invalidConnection":
    "작업 공간의 연결이 올바르지 않습니다.",
  "workspaces.actions.replace": "작업 공간 교체",
} as const;
