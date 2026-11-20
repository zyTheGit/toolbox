/** 翻译服务密钥键：与 SecretStatus / SecretPatch 的字段一一对应。 */
export type SecretKey = "openai" | "deepl" | "baidu" | "youdao";

/** 可按需安装的外部工具键，与后端 install_tool 命令一致。 */
export type ToolKey = "ytdlp" | "ffmpeg";
