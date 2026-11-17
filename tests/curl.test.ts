import assert from "node:assert/strict";
import test from "node:test";
import {
  parseCurl,
  parseHeadersText,
  toAxios,
  toCurl,
  toFetch,
  toPython,
  tokenizeCurl,
} from "../src/utils/curl.ts";

test("tokenizeCurl 处理引号、转义与续行", () => {
  const tokens = tokenizeCurl(
    "curl -H 'A: b c' \"https://x/y\" \\\n -d \"{\\\"a\\\":1}\" -d $'line\\nnext'",
  );
  assert.deepEqual(tokens, [
    "curl",
    "-H",
    "A: b c",
    "https://x/y",
    "-d",
    '{"a":1}',
    "-d",
    "line\nnext",
  ]);
  assert.throws(() => tokenizeCurl("curl 'oops"), /单引号未闭合/);
});

test("parseCurl 解析方法、请求头与请求体", () => {
  const request = parseCurl(
    `curl 'https://api.openai.com/v1/chat/completions' \\\n` +
      `  -H 'Content-Type: application/json' \\\n` +
      `  -H 'Authorization: Bearer sk-test' \\\n` +
      `  -d '{"model":"gpt-4o-mini"}'`,
  );
  assert.equal(request.method, "POST");
  assert.equal(request.url, "https://api.openai.com/v1/chat/completions");
  assert.deepEqual(request.headers, [
    { name: "Content-Type", value: "application/json" },
    { name: "Authorization", value: "Bearer sk-test" },
  ]);
  assert.equal(request.body, '{"model":"gpt-4o-mini"}');
});

test("parseCurl 推断 GET 并把 -G 数据挪到 query", () => {
  const get = parseCurl("curl https://example.com/api");
  assert.equal(get.method, "GET");
  assert.equal(get.body, "");

  const query = parseCurl("curl -G https://example.com/api -d 'a=1&b=2'");
  assert.equal(query.method, "GET");
  assert.equal(query.url, "https://example.com/api?a=1&b=2");
  assert.equal(query.body, "");
});

test("parseCurl 处理 --json、-u 与 -F", () => {
  const json = parseCurl("curl https://x.test --json '{\"a\":1}'");
  assert.equal(json.headers.find((item) => item.name === "Content-Type")?.value, "application/json");

  const auth = parseCurl("curl -u user:pass https://x.test");
  assert.equal(
    auth.headers.find((item) => item.name === "Authorization")?.value,
    "Basic dXNlcjpwYXNz",
  );

  const form = parseCurl("curl https://x.test -F name=value -F note=hello");
  assert.deepEqual(form.form, [
    { name: "name", value: "value" },
    { name: "note", value: "hello" },
  ]);
  assert.equal(form.method, "POST");
});

test("parseCurl 拒绝非法输入并提示本地文件", () => {
  assert.throws(() => parseCurl("wget https://x.test"), /curl/);
  assert.throws(() => parseCurl("curl"), /URL/);
  const withFile = parseCurl("curl https://x.test -d @payload.json");
  assert.equal(withFile.warnings.length, 1);
});

test("生成器输出包含关键结构", () => {
  const request = parseCurl(
    "curl https://x.test -X PUT -H 'X-Token: t' -d '{\"a\":1}'",
  );
  const fetchCode = toFetch(request);
  assert.match(fetchCode, /await fetch\("https:\/\/x\.test"/);
  assert.match(fetchCode, /method: "PUT"/);
  assert.match(fetchCode, /body: `\{"a":1\}`/);

  const axiosCode = toAxios(request);
  assert.match(axiosCode, /axios\.request/);
  assert.match(axiosCode, /data: `\{"a":1\}`/);

  const pythonCode = toPython(request);
  assert.match(pythonCode, /requests\.put\("https:\/\/x\.test"/);
  assert.match(pythonCode, /headers=headers/);
  assert.match(pythonCode, /data="\{\\"a\\":1\}"/);
});

test("toCurl 与 parseCurl 往返保持一致", () => {
  const original = parseCurl(
    "curl https://x.test/path -X POST -H 'Content-Type: application/json' -d '{\"name\":\"测试\"}'",
  );
  const rebuilt = parseCurl(toCurl(original));
  assert.equal(rebuilt.method, "POST");
  assert.equal(rebuilt.url, "https://x.test/path");
  assert.equal(rebuilt.body, '{"name":"测试"}');
  assert.deepEqual(rebuilt.headers, original.headers);
});

test("parseHeadersText 忽略空行与注释", () => {
  assert.deepEqual(parseHeadersText("A: 1\n\n# note\nB: two:three"), [
    { name: "A", value: "1" },
    { name: "B", value: "two:three" },
  ]);
});
