import assert from "node:assert/strict";
import test from "node:test";
import { PINYIN_CONF } from "../src/assets/conf/PINYIN_CONF.ts";
import { loadTypeScript } from "./helpers/load-typescript.mjs";

const { matchesText, searchWords, toSearchableName } = loadTypeScript(
  new URL("../src/modules/hosts/search.ts", import.meta.url),
  { "@/assets/conf/PINYIN_CONF.ts": { PINYIN_CONF } },
);

const matches = (text, query) =>
  matchesText(toSearchableName(text), searchWords(query));

test("中文名称支持全拼、首字母、部分音节和中文拼音混合", () => {
  for (const query of [
    "北京",
    "BEIJING",
    "bjfwq",
    "beij",
    "北jingfw",
    "服务器",
  ]) {
    assert.equal(matches("北京服务器", query), true, query);
  }
  assert.equal(matches("北京服务器", "shanghai"), false);
});

test("多音字匹配保留不同读音，不仅依赖第一读音", () => {
  for (const query of ["chongqing", "zhongqing", "cq", "zq", "重qing"]) {
    assert.equal(matches("重庆-01", query), true, query);
  }
});

test("地址用户名和数字保留直接匹配，多词须全部匹配", () => {
  assert.equal(matches("DB-Prod.Example.com", "prod.example"), true);
  assert.equal(matches("user@10.0.0.12:2222", "10.0.0.12 2222"), true);
  assert.equal(matches("北京生产服务器", "bj shengchan"), true);
  assert.equal(matches("北京生产服务器", "bj shanghai"), false);
  assert.equal(matches("北京服务器", "   "), true);
});
