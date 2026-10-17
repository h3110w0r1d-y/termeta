// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.sidebar.toggle": "Show or hide SFTP",
  "files.errors.sshDisconnected":
    "SSH disconnected. Refresh after reconnecting.",
  "files.count": "{count} items",
  "files.countPartial": "{count} items (more available)",
  "files.transfer.summary":
    "{name} · {completed}/{total} files · {transferred}{size}",
  "files.actions.upload": "Upload",
  "files.actions.parent": "Parent folder",
  "files.actions.download": "Download",
  "files.actions.home": "Home folder",
  "files.transfer.active": "Transferring",
  "files.transfer.progress": "Transferring {completed}/{total}",
  "files.transfer.complete": "Transfer complete",
  "files.transfer.queue": "Transfer queue",
  "files.types.other": "Other file",
  "files.copy.preparing": "Preparing to copy…",
  "files.errors.staleList": "File list is out of date. Refresh to update it.",
  "files.actions.go": "Go",
  "files.actions.loadMorePage": "Load more (100 items)",
  "files.browser.title": "Dual-pane SFTP browser",
  "files.actions.cancelTransfer": "Cancel transfer",
  "files.actions.cancelCopy": "Cancel copy",
  "files.copy.unsupportedType":
    "Only regular files and folders can be copied. Symbolic links are not supported yet.",
  "files.copy.dragHint": "Copy {count} items · Drag to the other pane",
  "files.copy.confirm": "Copy {count} items to the other folder",
  "files.actions.copyToOtherPane": "Copy to other pane",
  "files.fields.size": "Size",
  "files.copy.complete": "Copy complete",
  "files.upload.confirmDrop":
    "Upload the dropped files or folders to {target}? Folders with the same name will be merged. You will be asked before each existing file is overwritten.",
  "files.transfer.bytes": "Transferred {transferred} / {total}",
  "files.selection.loaded": "{loaded} items loaded · {selected} selected",
  "files.transfer.removed": "Removed from queue",
  "files.selection.count": "{count} items selected",
  "files.selection.named": "Selected: {name}",
  "files.transfer.queueFull":
    "The queue has reached 32 transfers. Wait for a transfer to finish.",
  "files.types.file": "File",
  "files.types.directory": "Folder",
  "files.types.directoryLink": "Folder symbolic link",
  "files.types.fileLink": "File symbolic link",
  "files.errors.tooManyEntries":
    "Too many files to display. Open a subfolder to continue browsing.",
  "files.confirmDisconnect":
    "Disconnecting SSH will cancel current SFTP transfers. Continue?",
  "files.actions.showHidden": "Show hidden files",
  "files.fields.permissions": "Permissions",
  "files.upload.dropHint": "Release to upload to {path}",
  "files.transfer.inProgress": "Transferring",
  "files.confirmCloseTerminal":
    "SFTP transfers are in progress. Closing will cancel them. Continue?",
  "files.sidebar.label": "SFTP browser for this terminal",
  "files.errors.dropBusy":
    "Wait for the current operation to finish, then drag the files again.",
  "files.path.placeholder": "Enter a remote path",
  "files.sidebar.resize": "Resize SFTP sidebar",
  "files.types.unknownLink": "Symbolic link (unknown target type)",
  "files.path.label": "Remote path",
  "files.transfer.connectionChanged":
    "Connection changed. Transfer was not started.",
  "files.hostPlaceholder": "Select a host to browse files",
  "files.transfer.preparing": "Selecting files or preparing transfer…",
  "files.sidebar.hide": "Hide SFTP sidebar",
  "files.transfer.hidePanel": "Hide panel and continue transfers",
} as const;
