// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "files.hosts.search": "Search SSH hosts…",
  "files.hosts.empty": "No saved SSH hosts. Add one in the vault first.",
  "files.drag.pastePaths": "Release to paste {count} paths",
  "files.drag.hint":
    "{count} items · Drop on the terminal to paste paths, or outside the window to download",
  "files.drag.invalidPaths":
    "The selected paths are too long or contain unsupported characters. Select fewer items and try again.",
  "files.follow.label": "Follow terminal directory",
  "files.follow.description":
    "Sync SFTP when the terminal reports a directory change. Follow the latest directory after file operations finish.",
  "files.follow.waiting":
    "Waiting for the terminal directory. The shell must report it or include the full path in its title.",
  "files.navigation.collapsed": "Show hidden parent folders",
  "files.navigation.edit": "Edit folder path",
  "files.activity.title": "Transfers",
  "files.activity.open": "View transfer list",
  "files.activity.active": "{count} transfers active",
  "files.activity.clearFinished": "Clear finished",
  "files.activity.empty": "No transfers yet",
  "files.activity.idle": "All transfers have finished",
  "files.activity.failed": "Transfer failed",
  "files.activity.cancelling": "Cancelling…",
  "files.activity.progress": "Transfer progress",
  "files.activity.unknownProgress":
    "Preparing; total progress is not available yet",
  "files.table.label": "Remote files",
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
  "files.manage.delete.action": "Delete",
  "files.manage.delete.title": "Delete selected items",
  "files.manage.delete.recursiveTitle":
    "Confirm deletion of folders and contents",
  "files.manage.delete.warning":
    "These items will be permanently deleted from the remote host. This cannot be undone. Only the links themselves are deleted.",
  "files.manage.delete.recursiveWarning":
    "Selected folders and all their contents, including hidden files, will be permanently deleted. If interrupted or unsuccessful, items already deleted cannot be recovered. Delete recursively?",
  "files.manage.delete.continue": "Continue to folder confirmation",
  "files.manage.delete.permanent": "Delete permanently",
  "files.manage.selection": "{count} items selected",
  "files.manage.partialFailure":
    "Some changes may already have completed. Review the refreshed directory before retrying; completed changes are not rolled back.",
  "files.manage.limit": "Select no more than 100 items per operation.",
  "files.manage.permissions.title": "Edit permissions",
  "files.manage.permissions.applies":
    "Apply these permissions to every selected item. Folder contents are not changed recursively. Set an explicit value when current permissions differ or are unknown.",
  "files.manage.permissions.owner": "Owner",
  "files.manage.permissions.group": "Group",
  "files.manage.permissions.others": "Others",
  "files.manage.permissions.read": "Read",
  "files.manage.permissions.write": "Write",
  "files.manage.permissions.execute": "Execute / enter",
  "files.manage.permissions.code": "Octal permissions",
  "files.manage.permissions.invalid":
    "Enter 3 or 4 octal digits (000 to 7777).",
  "files.manage.permissions.special": "Special permissions (advanced)",
  "files.manage.permissions.setuid": "Setuid: execute as owner",
  "files.manage.permissions.setgid": "Setgid: inherit group / execute as group",
  "files.manage.permissions.sticky":
    "Sticky: restrict deletion of directory entries to their owners",
  "files.manage.delete.confirmContents":
    "I understand that the selected folders and all their contents will be permanently deleted.",
} as const;
