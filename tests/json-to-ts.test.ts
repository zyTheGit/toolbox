import assert from "node:assert/strict";
import test from "node:test";
import { jsonToTs } from "../src/utils/json-to-ts.ts";

test("根对象产出 interface，且不重复声明 Root", () => {
  const output = jsonToTs({ name: "Toolbox", version: 1 });
  assert.equal(output, "interface Root {\n  name: string;\n  version: number;\n}");
  assert.doesNotMatch(output, /type Root = Root/);
});

test("嵌套对象按路径命名", () => {
  const output = jsonToTs({ user_info: { nick_name: "a" }, vip: true });
  assert.match(output, /interface UserInfo \{/);
  assert.match(output, /interface Root \{\n  user_info: UserInfo;\n  vip: boolean;\n\}/);
});

test("对象数组合并为一个 interface，缺字段标记可选", () => {
  const output = jsonToTs({
    users: [
      { id: 1, name: "a" },
      { id: 2, email: "b@c.d" },
    ],
  });
  assert.match(output, /interface User \{/);
  assert.match(output, /  name\?: string;/);
  assert.match(output, /  email\?: string;/);
  assert.match(output, /  id: number;/);
  assert.match(output, /users: User\[\];/);
});

test("联合类型作为数组元素时补括号", () => {
  const output = jsonToTs({ list: [1, "a", 2] });
  assert.equal(output, "interface Root {\n  list: (number | string)[];\n}");
});

test("混合类型数组（对象与基础类型）同样补括号", () => {
  const output = jsonToTs({ list: [{ a: 1 }, null] });
  assert.match(output, /list: \(ListItem \| null\)\[\];/);
});

test("空数组与空对象", () => {
  assert.equal(jsonToTs({ empty: [], obj: {} }), "interface Root {\n  empty: unknown[];\n  obj: Record<string, unknown>;\n}");
});

test("根为数组时产出类型别名", () => {
  const output = jsonToTs([{ id: 1 }, { id: 2 }]);
  assert.match(output, /interface RootItem \{/);
  assert.match(output, /type Root = RootItem\[\];$/);
});

test("根为基础类型时产出类型别名", () => {
  assert.equal(jsonToTs("text"), "type Root = string;");
  assert.equal(jsonToTs(null), "type Root = null;");
});

test("非法属性名加引号", () => {
  const output = jsonToTs({ "user-name": "a", "1st": 2 });
  assert.match(output, /"user-name": string;/);
  assert.match(output, /"1st": number;/);
});

test("重名子对象自动加后缀，互不覆盖", () => {
  // a_b 与 aB 都会转换成 AB，后者需要加后缀，否则声明互相覆盖
  const output = jsonToTs({
    a_b: { x: 1 },
    aB: { y: 2 },
  });
  assert.match(output, /interface AB \{/);
  assert.match(output, /interface AB2 \{/);
  assert.match(output, /a_b: AB;/);
  assert.match(output, /aB: AB2;/);
});

test("结构相同但路径不同也各自命名，不产生重复声明", () => {
  const output = jsonToTs({ first: { v: 1 }, second: { v: 2 } });
  assert.match(output, /interface First \{/);
  assert.match(output, /interface Second \{/);
  assert.equal(new Set(output.match(/interface \w+/g)).size, 3);
});

test("中文属性名可作类型名，不加引号", () => {
  const output = jsonToTs({ 用户信息: { 昵称: "a" } });
  assert.match(output, /interface 用户信息 \{/);
  assert.match(output, /  用户信息: 用户信息;/);
});

test("空数组不让位覆盖具体元素类型", () => {
  const output = jsonToTs({ rows: [{ tags: ["x"] }, { tags: [] }] });
  assert.match(output, /  tags: string\[\];/);
});

test("合并数组中的 null 与具体类型", () => {
  const output = jsonToTs({ rows: [{ v: "a" }, { v: null }] });
  assert.match(output, /  v: string \| null;/);
});

test("接口嵌套与可选字段的组合输出", () => {
  const output = jsonToTs({
    code: 0,
    data: {
      total: 2,
      users: [
        { id: 1, name: "a", profile: { age: 1 } },
        { id: 2, name: "b" },
      ],
    },
  });
  assert.equal(
    output,
    [
      "interface UserProfile {",
      "  age: number;",
      "}",
      "",
      "interface User {",
      "  id: number;",
      "  name: string;",
      "  profile?: UserProfile;",
      "}",
      "",
      "interface Data {",
      "  total: number;",
      "  users: User[];",
      "}",
      "",
      "interface Root {",
      "  code: number;",
      "  data: Data;",
      "}",
    ].join("\n"),
  );
});

test("null 字段保留 null 类型", () => {
  assert.match(jsonToTs({ avatar: null }), /  avatar: null;/);
});
