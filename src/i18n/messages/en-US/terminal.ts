// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "terminal.regions.area": "Terminal area",
  "terminal.search.label": "Search terminal history",
  "terminal.search.placeholder": "Search terminal history…",
  "terminal.search.close": "Close search",
  "terminal.actions.copySelection": "Copy selection",
  "terminal.actions.paste": "Paste",
  "terminal.actions.find": "Find in terminal",
  "terminal.actions.scrollToBottom": "Scroll to bottom",
  "terminal.actions.openLink": "Open link",
  "terminal.titles.local": "Local terminal",
  "terminal.snapshot.header":
    "\r\n[Full-screen snapshot before disconnect]\r\n",
  "terminal.auth.title": "SSH authentication",
  "terminal.snapshot.truncated": "[Full-screen snapshot truncated at 1 MiB]",
  "terminal.errors.unsentInput":
    "{error}; unsent input will not be retried automatically.",
  "terminal.actions.closeNamed": "Close terminal {name}",
  "terminal.errors.fontLoad":
    "Terminal font failed to load. Apply your settings again or restart the app.",
  "terminal.state.created": "Ready to connect",
  "terminal.paste.tooLarge": "A single paste cannot exceed 64 KiB.",
  "terminal.actions.cancelConnection": "Cancel connection",
  "terminal.state.startingLocal": "Starting local terminal",
  "terminal.titles.command": "Command",
  "terminal.state.commandFinished":
    "Command finished. Use the context menu to run it again.",
  "terminal.auth.answerTooLong": "Answers cannot exceed 16,384 characters",
  "terminal.paste.broadcastDescription":
    "Paste {lines} lines into {count} terminals",
  "terminal.paste.currentDescription": "Paste {lines} lines into this terminal",
  "terminal.state.closed": "Connection closed",
  "terminal.state.disconnected": "Disconnected",
  "terminal.state.connected": "Connected",
  "terminal.state.startingSsh": "Starting SSH connection",
  "terminal.state.startingTelnet": "Starting Telnet connection (unencrypted)",
  "terminal.state.opening": "Opening terminal",
  "terminal.paste.warning":
    "Line breaks may execute remote commands immediately. Review all content before continuing.",
  "terminal.search.history": "Search terminal history",
  "terminal.actions.disconnect": "Disconnect",
  "terminal.errors.outputRead":
    "Unable to read terminal output. Reconnect to continue.",
  "terminal.errors.sessionClosing":
    "The previous connection is still closing. Try again shortly.",
  "terminal.errors.inputUnavailable":
    "Unable to send input. Reconnect to continue.",
  "terminal.state.closing": "Disconnecting",
  "terminal.state.unavailable": "Connecting or connection ended",
  "terminal.paste.confirmMultiline": "Confirm multiline paste",
  "terminal.paste.confirmBroadcast": "Broadcast to {targetCount} terminals",
  "terminal.paste.confirmCurrent": "Paste into this terminal",
  "terminal.state.awaitingHostKey": "Awaiting host verification",
  "terminal.state.awaitingAuth": "Awaiting authentication",
  "terminal.titles.terminal": "Terminal",
  "terminal.errors.outputPending":
    "The terminal is still processing output. Try again shortly. Your history has been preserved.",
  "terminal.errors.outputFailure":
    "Terminal output was interrupted. Reconnect to continue.",
  "terminal.errors.tcpConnectFailed": "Unable to connect to the host and port.",
  "terminal.form.theme": "Terminal color scheme",
  "terminal.accessibility.title": "Terminal: {name}",
  "terminal.state.authenticating": "Authenticating",
  "terminal.theme.hostDefault": "Use host / default color scheme",
  "terminal.paste.queueFull":
    "Input is busy. Wait a moment before pasting again.",
  "terminal.state.connecting": "Connecting",
  "terminal.state.failed": "Connection failed",
  "terminal.state.ended": "Connection ended",
  "terminal.auth.target": "Connecting to: {targetTitle}",
  "terminal.state.incompleteOutput":
    "Connection ended. The final output may be incomplete.",
  "terminal.actions.reopenLocal":
    "Reopening local terminal. Terminal history is preserved.",
  "terminal.actions.rerun": "Run again",
  "terminal.actions.reconnect": "Reconnect",
  "terminal.actions.reconnectWithHistory":
    "Reconnecting. Terminal history is preserved.",
  "terminal.exit.signal": " (signal {exitSignal})",
  "terminal.exit.code": " (exit code {exitCode})",
} as const;
