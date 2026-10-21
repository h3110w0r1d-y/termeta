import assert from "node:assert/strict";
import test from "node:test";
import {
  parseWorkingDirectory,
  sftpWorkingDirectory,
  updateWorkingDirectory,
} from "../src/modules/terminal/workingDirectory.ts";

test("目录上报解码 Unicode 与空格，拒绝非文件 URI、控制字符和非法路径", () => {
  assert.equal(
    parseWorkingDirectory(
      "file://remote/home/%E6%96%87%E4%BB%B6/a%20b",
      "osc7",
    ),
    "/home/文件/a b",
  );
  assert.equal(parseWorkingDirectory("file:///tmp/%2520", "osc7"), "/tmp/%20");
  assert.equal(
    parseWorkingDirectory("CurrentDir=/var/log", "osc1337"),
    "/var/log",
  );
  for (const uri of [
    "https://remote/path",
    "file://remote/tmp/%ZZ",
    "file://remote/tmp/%00",
    "file://remote/tmp/%5Ctest",
    "file://remote/tmp/test\n",
    "file://remote/tmp\\test",
    "file://remote/tmp?query=1",
    "file://remote/tmp#fragment",
    "file://user@remote/tmp",
  ])
    assert.equal(parseWorkingDirectory(uri, "osc7"), undefined, uri);
  assert.equal(
    parseWorkingDirectory("CurrentDir=relative/path", "osc1337"),
    undefined,
  );
  assert.equal(
    parseWorkingDirectory(`CurrentDir=/${"文".repeat(1400)}`, "osc1337"),
    undefined,
  );
});

test("标题兼容完整绝对路径与主目录，不从命令或截断标题猜目录", () => {
  assert.equal(
    parseWorkingDirectory("root@host:/var/log", "title"),
    "/var/log",
  );
  assert.equal(
    parseWorkingDirectory("user@host:~/项目 a", "title"),
    "~/项目 a",
  );
  assert.equal(parseWorkingDirectory("user@host:~", "title"), "~");
  for (const title of [
    "/var/log",
    "vim /tmp/a",
    "user@host:repo",
    "user@host:/…/log",
    "user@host:~/.../log",
  ])
    assert.equal(parseWorkingDirectory(title, "title"), undefined);
  assert.equal(sftpWorkingDirectory("~"), ".");
  assert.equal(sftpWorkingDirectory("~/项目 a"), "./项目 a");
  assert.equal(sftpWorkingDirectory("~//etc"), ".//etc");
  assert.equal(sftpWorkingDirectory("/var/log"), "/var/log");
});

test("明确目录上报优先于标题，非法上报不污染有效目录", () => {
  const terminal = {};
  updateWorkingDirectory(terminal, "user@host:~", "title");
  assert.equal(terminal.workingDirectory, "~");
  updateWorkingDirectory(terminal, "file://host/var/log", "osc7");
  updateWorkingDirectory(terminal, "user@host:/tmp", "title");
  updateWorkingDirectory(terminal, "file://host/%00", "osc7");
  assert.equal(terminal.workingDirectory, "/var/log");
  updateWorkingDirectory(terminal, "CurrentDir=/srv/app", "osc1337");
  assert.equal(terminal.workingDirectory, "/srv/app");
});
