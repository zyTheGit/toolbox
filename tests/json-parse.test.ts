import assert from "node:assert/strict";
import test from "node:test";
import { parseJsonWithLocation } from "../src/utils/json-parse.ts";

/** 断言解析失败，并返回错误对象 */
function expectError(text: string) {
  const result = parseJsonWithLocation(text);
  assert.equal(result.ok, false, `预期解析失败，实际成功：${text}`);
  if (result.ok) throw new Error("unreachable");
  return result.error;
}

test("合法 JSON 正常解析", () => {
  const result = parseJsonWithLocation('{"a":[1,2],"b":null,"c":true}');
  assert.equal(result.ok, true);
  if (result.ok) assert.deepEqual(result.value, { a: [1, 2], b: null, c: true });
});

test("转义与 Unicode 正常解析", () => {
  const result = parseJsonWithLocation('"a\\"b\\\\c\\u4e2d\\n"');
  assert.equal(result.ok, true);
  if (result.ok) assert.equal(result.value, 'a"b\\c中\n');
});

test("根值为 null 不算失败", () => {
  const result = parseJsonWithLocation("null");
  assert.equal(result.ok, true);
  if (result.ok) assert.equal(result.value, null);
});

test("对象尾随逗号定位到右括号", () => {
  const error = expectError('{"a":1,}');
  assert.match(error.message, /尾随逗号/);
  assert.equal(error.line, 1);
  assert.equal(error.column, 8);
});

test("数组尾随逗号定位到右方括号", () => {
  const error = expectError("[1,]");
  assert.match(error.message, /尾随逗号/);
  assert.equal(error.column, 4);
});

test("缺少逗号时指向下一个属性", () => {
  const error = expectError('{"a":1 "b":2}');
  assert.match(error.message, /缺少逗号/);
  assert.equal(error.column, 8);
});

test("多层缩进的错误能报出行列", () => {
  const error = expectError('{\n  "a": 1,\n  "b": 2,\n}');
  assert.match(error.message, /尾随逗号/);
  assert.equal(error.line, 4);
  assert.equal(error.column, 1);
  assert.match(error.snippet, /^\}\n\^$/);
});

test("CRLF 换行同样能算行号", () => {
  const error = expectError('{\r\n  "a": 1,\r\n}');
  assert.equal(error.line, 3);
  assert.equal(error.column, 1);
});

test("字符串未闭合指向起始引号", () => {
  const error = expectError('{"a":"x}');
  assert.match(error.message, /字符串未闭合/);
  assert.equal(error.column, 6);
});

test("单引号给出双引号提示", () => {
  const error = expectError("{'a':1}");
  assert.match(error.message, /双引号/);
  assert.equal(error.column, 2);
});

test("注释给出明确提示", () => {
  const error = expectError('{"a":1 // 注释\n}');
  assert.match(error.message, /不支持注释/);
  assert.equal(error.column, 8);
});

test("非法字面量", () => {
  const error = expectError('{"a":tru}');
  assert.match(error.message, /无法识别的值/);
  assert.equal(error.column, 6);
});

test("前导零数字", () => {
  const error = expectError('{"a":01}');
  assert.match(error.message, /前导零/);
  assert.equal(error.column, 7);
});

test("指数不完整", () => {
  assert.match(expectError('{"a":1e}').message, /指数部分不完整/);
});

test("值结束后有多余内容", () => {
  const error = expectError('{"a":1} x');
  assert.match(error.message, /多余内容/);
  assert.equal(error.column, 9);
});

test("对象缺少右括号时报文件结尾", () => {
  const error = expectError('{"a":1');
  assert.match(error.message, /对象缺少 '\}'/);
  assert.equal(error.column, 7);
});

test("数组缺少右括号时报文件结尾", () => {
  assert.match(expectError("[1,2").message, /数组缺少 '\]'/);
});

test("字符串中的裸换行", () => {
  const error = expectError('{"a":"x\ny"}');
  assert.match(error.message, /控制字符/);
  assert.equal(error.line, 1);
});

test("属性名后缺少冒号", () => {
  assert.match(expectError('{"a" 1}').message, /缺少 ':'/);
});

test("空内容", () => {
  assert.match(expectError("   ").message, /内容为空/);
});

test("空对象与空数组是合法 JSON", () => {
  const result = parseJsonWithLocation("{}");
  assert.equal(result.ok, true);
  if (result.ok) assert.deepEqual(result.value, {});
  assert.deepEqual(parseJsonWithLocation("[]"), { ok: true, value: [] });
});

test("snippet 用制表符宽度对齐指示符", () => {
  const error = expectError('\t"a": 1,');
  // 制表符渲染为两个空格，报错位置落在 ':' 上（偏移 4，制表符占 2 列）
  assert.equal(error.snippet, '  "a": 1,\n     ^');
});
