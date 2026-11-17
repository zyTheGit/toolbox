import assert from "node:assert/strict";
import test from "node:test";
import { convertCase } from "../src/utils/text-case.ts";

test("全部大写 / 全部小写", () => {
  assert.equal(convertCase("Hello World", "upper"), "HELLO WORLD");
  assert.equal(convertCase("Hello World", "lower"), "hello world");
});

test("反转大小写", () => {
  assert.equal(convertCase("Hello World 123", "swap"), "hELLO wORLD 123");
});

test("单词首字母大写", () => {
  assert.equal(convertCase("hello world", "title"), "Hello World");
  assert.equal(convertCase("user_profile-name", "title"), "User_Profile-Name");
});

test("句首大写", () => {
  assert.equal(
    convertCase("hello world. next sentence! third?", "sentence"),
    "Hello world. Next sentence! Third?",
  );
});

test("中文内容不受首字母转换影响", () => {
  assert.equal(convertCase("你好 world", "title"), "你好 World");
});

test("命名风格转换的切词", () => {
  assert.equal(convertCase("user profile info", "camel"), "userProfileInfo");
  assert.equal(convertCase("UserProfileInfo", "camel"), "userProfileInfo");
  assert.equal(convertCase("user_profile-info", "pascal"), "UserProfileInfo");
  assert.equal(convertCase("userProfileInfo", "snake"), "user_profile_info");
  assert.equal(convertCase("userProfileInfo", "kebab"), "user-profile-info");
  assert.equal(convertCase("userProfileInfo", "constant"), "USER_PROFILE_INFO");
});

test("连续大写缩写能正确切分", () => {
  assert.equal(convertCase("HTTPResponseCode", "snake"), "http_response_code");
  assert.equal(convertCase("XML_HTTP_request", "camel"), "xmlHttpRequest");
});

test("纯中文无法切词时原样返回", () => {
  assert.equal(convertCase("用户信息", "camel"), "用户信息");
});

test("数字参与切词", () => {
  assert.equal(convertCase("user2Name", "kebab"), "user2-name");
});
