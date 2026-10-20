import common from "./common";
import vault from "./vault";
import window from "./window";
import terminal from "./terminal";
import files from "./files";
import shortcuts from "./shortcuts";
import workbench from "./workbench";
import settings from "./settings";
import json from "./json";
import hosts from "./hosts";
import knownHosts from "./knownHosts";
import workspaces from "./workspaces";
import serial from "./serial";
import credentials from "./credentials";
import sessionLogs from "./sessionLogs";
import snippets from "./snippets";
import forwarding from "./forwarding";
import themes from "./themes";
import backup from "./backup";
import account from "./account";

import nativeMenu from "./nativeMenu.json";

export default {
  ...nativeMenu,
  ...common,
  ...vault,
  ...window,
  ...terminal,
  ...files,
  ...shortcuts,
  ...workbench,
  ...settings,
  ...json,
  ...hosts,
  ...knownHosts,
  ...workspaces,
  ...serial,
  ...credentials,
  ...sessionLogs,
  ...snippets,
  ...forwarding,
  ...themes,
  ...backup,
  ...account,
} as const;
