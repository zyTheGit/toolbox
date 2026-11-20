/* 数据库视图设计稿种子数据：按引擎 / 数据库组织 schema 与示例行 */

const TASK_NAMES = ["每日站会提醒", "清理下载缓存", "每周备份数据库", "同步翻译历史", "检查工具更新", "晨报推送", "日志归档", "缓存预热"];
const CRONS = ["每天 09:00", "每周一 03:30", "每周日 22:00", "每小时", "每天 08:00", "每月 1 日 00:00"];
const STATUSES = ["success", "success", "success", "failed", null];

const taskRows = (n = 64) => Array.from({ length: n }, (_, i) => [
  i + 1,
  TASK_NAMES[i % TASK_NAMES.length] + (i >= TASK_NAMES.length ? ` ${Math.floor(i / TASK_NAMES.length) + 1}` : ""),
  CRONS[i % CRONS.length],
  i % 5 === 4 ? 0 : 1,
  STATUSES[i % STATUSES.length],
]);
const downloadRows = (n = 30) => Array.from({ length: n }, (_, i) => [
  i + 1,
  `https://example.com/video/${1000 + i}`,
  ["completed", "completed", "failed", "canceled"][i % 4],
  ["1080p", "720p", "最佳 MP4", "仅音频 MP3"][i % 4],
  i % 4 === 2 ? null : `D:\\Downloads\\Toolbox\\video-${1000 + i}.mp4`,
]);

const APP_DATA = {
  tasks: { cols: ["id", "name", "cron", "enabled", "last_status"], rows: taskRows() },
  downloads: { cols: ["id", "url", "status", "quality", "path"], rows: downloadRows() },
  translate_history: {
    cols: ["id", "src", "dst", "provider", "fav"],
    rows: [
      [1, "桌面工具箱把翻译放在托盘里。", "The desktop toolbox keeps translation in the tray.", "微软翻译", 1],
      [2, "The queue resumes after restart.", "下载队列在重启后自动恢复。", "Google", 0],
      [3, "凭据保存在本机。", "Credentials are stored locally.", "微软翻译", 1],
      [4, "Only read-only statements are allowed.", "仅允许只读语句。", "DeepL", 0],
      [5, "每周备份数据库", "Back up the database weekly", "微软翻译", 0],
      [6, "Fail over to the next provider.", "失败时回退到下一个服务。", "Google", 1],
      [7, "分片并发下载线程", "Concurrent segment download threads", "微软翻译", 0],
      [8, "SHA256 verification is fail-closed.", "SHA256 校验采用 fail-closed。", "DeepL", 0],
      [9, "清理下载缓存", "Clean the download cache", "微软翻译", 0],
      [10, "The tray icon shows live progress.", "托盘图标显示实时进度。", "Google", 1],
      [11, "实验性功能默认关闭。", "Experimental features are off by default.", "微软翻译", 0],
      [12, "Portable builds keep data beside the exe.", "便携版数据放在 exe 旁。", "DeepL", 0],
    ],
  },
  settings: {
    cols: ["key", "value"],
    rows: [
      ["download.dir", "D:\\Downloads\\Toolbox"], ["download.concurrent", "2"], ["download.threads", "8"],
      ["translate.provider", "microsoft"], ["translate.fallback", "true"], ["startup.autostart", "true"],
      ["shell.exec_enabled", "false"], ["notify.enabled", "true"],
    ],
  },
  v_task_stats: {
    cols: ["name", "runs", "fail_rate"],
    rows: TASK_NAMES.map((n, i) => [n, 40 - i * 3, i % 3 === 0 ? 0.12 : i % 3 === 1 ? 0 : 0.05]),
  },
};

const colsOf = (names) => names.map(([n, t, pk]) => [n, t, pk]);
const SQLITE_SCHEMA = [
  { name: "tasks", kind: "T", cols: colsOf([["id", "INTEGER", true], ["name", "TEXT"], ["cron", "TEXT"], ["enabled", "INTEGER"], ["last_status", "TEXT"], ["created_at", "TEXT"]]) },
  { name: "downloads", kind: "T", cols: colsOf([["id", "INTEGER", true], ["url", "TEXT"], ["status", "TEXT"], ["quality", "TEXT"], ["path", "TEXT"], ["created_at", "TEXT"]]) },
  { name: "translate_history", kind: "T", cols: colsOf([["id", "INTEGER", true], ["src", "TEXT"], ["dst", "TEXT"], ["provider", "TEXT"], ["fav", "INTEGER"]]) },
  { name: "settings", kind: "T", cols: colsOf([["key", "TEXT", true], ["value", "TEXT"]]) },
  { name: "v_task_stats", kind: "V", cols: colsOf([["name", "TEXT"], ["runs", "INTEGER"], ["fail_rate", "REAL"]]) },
];
const MYSQL_TOOLBOX_SCHEMA = [
  { name: "tasks", kind: "T", cols: colsOf([["id", "INT", true], ["name", "VARCHAR(120)"], ["cron", "VARCHAR(40)"], ["enabled", "TINYINT(1)"], ["last_status", "VARCHAR(20)"], ["created_at", "DATETIME"]]) },
  { name: "downloads", kind: "T", cols: colsOf([["id", "INT", true], ["url", "VARCHAR(500)"], ["status", "VARCHAR(20)"], ["quality", "VARCHAR(20)"], ["path", "VARCHAR(500)"], ["created_at", "DATETIME"]]) },
  { name: "translate_history", kind: "T", cols: colsOf([["id", "INT", true], ["src", "TEXT"], ["dst", "TEXT"], ["provider", "VARCHAR(40)"], ["fav", "TINYINT(1)"]]) },
  { name: "settings", kind: "T", cols: colsOf([["key", "VARCHAR(60)", true], ["value", "TEXT"]]) },
  { name: "v_task_stats", kind: "V", cols: colsOf([["name", "VARCHAR(120)"], ["runs", "INT"], ["fail_rate", "DECIMAL(4,2)"]]) },
];

const POST_TITLES = ["Tauri 2 托盘应用实战", "m3u8 下载器设计笔记", "Vue 3 组合式 API 清单", "Rust 错误处理惯例", "SQLite 只读浏览方案", "前端字阶收敛实践", "图标统一 SVG sprite", "定时任务 cron 速查", "NSIS 安装包踩坑", "跨平台路径处理", "流式哈希校验实现", "托盘快捷翻译交互"];
const BLOG_DATA = {
  posts: {
    cols: ["id", "title", "author_id", "status", "views", "published_at"],
    rows: Array.from({ length: 42 }, (_, i) => [
      i + 1, POST_TITLES[i % POST_TITLES.length] + (i >= POST_TITLES.length ? `（${Math.floor(i / POST_TITLES.length) + 1}）` : ""),
      (i % 4) + 1, ["published", "published", "draft"][i % 3], 120 + ((i * 37) % 900),
      i % 3 === 2 ? null : `2026-1${i % 2}-0${(i % 9) + 1} 08:30:00`,
    ]),
  },
  comments: {
    cols: ["id", "post_id", "author", "body", "created_at"],
    rows: Array.from({ length: 120 }, (_, i) => [
      i + 1, (i % 42) + 1, ["访客甲", "访客乙", "路人丙", "读者丁"][i % 4],
      ["写得很清楚，收藏了。", "请问便携版数据目录在哪？", "实测通过，感谢。", "期待下一篇。"][i % 4],
      `2026-11-${String((i % 20) + 1).padStart(2, "0")} 1${i % 10}:24:00`,
    ]),
  },
  tags: {
    cols: ["id", "name", "slug"],
    rows: [["Tauri", "tauri"], ["Rust", "rust"], ["Vue", "vue"], ["下载", "download"], ["翻译", "translate"], ["定时任务", "cron"], ["打包", "packaging"], ["跨平台", "cross-platform"], ["安全", "security"], ["设计", "design"]].map(([n, s], i) => [i + 1, n, s]),
  },
  post_tags: {
    cols: ["post_id", "tag_id"],
    rows: Array.from({ length: 60 }, (_, i) => [(i % 42) + 1, (i % 10) + 1]),
  },
  v_post_stats: {
    cols: ["title", "comments", "views"],
    rows: POST_TITLES.map((t, i) => [t, 12 - i, 900 - i * 40]),
  },
};
const BLOG_SCHEMA = [
  { name: "posts", kind: "T", cols: colsOf([["id", "INT", true], ["title", "VARCHAR(200)"], ["author_id", "INT"], ["status", "VARCHAR(20)"], ["views", "INT"], ["published_at", "DATETIME"]]) },
  { name: "comments", kind: "T", cols: colsOf([["id", "INT", true], ["post_id", "INT"], ["author", "VARCHAR(60)"], ["body", "TEXT"], ["created_at", "DATETIME"]]) },
  { name: "tags", kind: "T", cols: colsOf([["id", "INT", true], ["name", "VARCHAR(40)"], ["slug", "VARCHAR(60)"]]) },
  { name: "post_tags", kind: "T", cols: colsOf([["post_id", "INT", true], ["tag_id", "INT", true]]) },
  { name: "v_post_stats", kind: "V", cols: colsOf([["title", "VARCHAR(200)"], ["comments", "INT"], ["views", "INT"]]) },
];

const PRODUCT_NAMES = ["机械键盘", "无线鼠标", "显示器支架", "USB-C 扩展坞", "降噪耳机", "桌面音箱", "护眼台灯", "高清摄像头", "固态硬盘", "内存条", "笔记本支架", "理线槽"];
const SHOP_DATA = {
  orders: {
    cols: ["id", "order_no", "customer_id", "amount", "status", "created_at"],
    rows: Array.from({ length: 80 }, (_, i) => [
      i + 1, `SO202611${String(i + 1).padStart(4, "0")}`, (i % 30) + 1,
      Number((49 + ((i * 53) % 900) + 0.9).toFixed(2)),
      ["paid", "shipped", "completed", "refunded"][i % 4],
      `2026-11-${String((i % 20) + 1).padStart(2, "0")} 10:0${i % 10}:00`,
    ]),
  },
  order_items: {
    cols: ["id", "order_id", "product_id", "qty", "price"],
    rows: Array.from({ length: 160 }, (_, i) => [i + 1, (i % 80) + 1, (i % 24) + 1, (i % 3) + 1, Number((29 + ((i * 31) % 400) + 0.5).toFixed(2))]),
  },
  products: {
    cols: ["id", "name", "price", "stock"],
    rows: Array.from({ length: 24 }, (_, i) => [i + 1, PRODUCT_NAMES[i % PRODUCT_NAMES.length] + (i >= PRODUCT_NAMES.length ? " Pro" : ""), Number((29 + i * 17 + 0.9).toFixed(2)), i % 7 === 0 ? 0 : 20 + i * 3]),
  },
  customers: {
    cols: ["id", "name", "city", "vip"],
    rows: Array.from({ length: 30 }, (_, i) => [i + 1, ["张伟", "王芳", "李娜", "刘洋", "陈静", "杨帆"][i % 6] + (i >= 6 ? ` ${Math.floor(i / 6) + 1}` : ""), ["杭州", "上海", "北京", "深圳", "成都"][i % 5], i % 4 === 0 ? 1 : 0]),
  },
  v_order_summary: {
    cols: ["customer", "orders", "total"],
    rows: Array.from({ length: 30 }, (_, i) => [["张伟", "王芳", "李娜", "刘洋", "陈静", "杨帆"][i % 6], 8 - (i % 5), Number((300 + i * 45.5).toFixed(2))]),
  },
};
const SHOP_SCHEMA = [
  { name: "orders", kind: "T", cols: colsOf([["id", "INT", true], ["order_no", "VARCHAR(24)"], ["customer_id", "INT"], ["amount", "DECIMAL(10,2)"], ["status", "VARCHAR(20)"], ["created_at", "DATETIME"]]) },
  { name: "order_items", kind: "T", cols: colsOf([["id", "INT", true], ["order_id", "INT"], ["product_id", "INT"], ["qty", "INT"], ["price", "DECIMAL(10,2)"]]) },
  { name: "products", kind: "T", cols: colsOf([["id", "INT", true], ["name", "VARCHAR(120)"], ["price", "DECIMAL(10,2)"], ["stock", "INT"]]) },
  { name: "customers", kind: "T", cols: colsOf([["id", "INT", true], ["name", "VARCHAR(60)"], ["city", "VARCHAR(40)"], ["vip", "TINYINT(1)"]]) },
  { name: "v_order_summary", kind: "V", cols: colsOf([["customer", "VARCHAR(60)"], ["orders", "INT"], ["total", "DECIMAL(12,2)"]]) },
];

const MYSQL_DBS = {
  toolbox: { schema: MYSQL_TOOLBOX_SCHEMA, data: APP_DATA },
  blog: { schema: BLOG_SCHEMA, data: BLOG_DATA },
  shop_demo: { schema: SHOP_SCHEMA, data: SHOP_DATA },
  information_schema: {
    schema: [
      { name: "TABLES", kind: "V", cols: colsOf([["TABLE_SCHEMA", "VARCHAR(64)"], ["TABLE_NAME", "VARCHAR(64)"], ["TABLE_TYPE", "VARCHAR(20)"], ["ENGINE", "VARCHAR(20)"], ["TABLE_ROWS", "BIGINT"]]) },
      { name: "COLUMNS", kind: "V", cols: colsOf([["TABLE_SCHEMA", "VARCHAR(64)"], ["TABLE_NAME", "VARCHAR(64)"], ["COLUMN_NAME", "VARCHAR(64)"], ["DATA_TYPE", "VARCHAR(40)"], ["IS_NULLABLE", "VARCHAR(3)"]]) },
      { name: "SCHEMATA", kind: "V", cols: colsOf([["SCHEMA_NAME", "VARCHAR(64)"], ["DEFAULT_CHARACTER_SET_NAME", "VARCHAR(32)"]]) },
    ],
    data: {
      TABLES: {
        cols: ["TABLE_SCHEMA", "TABLE_NAME", "TABLE_TYPE", "ENGINE", "TABLE_ROWS"],
        rows: [
          ["toolbox", "tasks", "BASE TABLE", "InnoDB", 64], ["toolbox", "downloads", "BASE TABLE", "InnoDB", 30],
          ["toolbox", "v_task_stats", "VIEW", null, null], ["blog", "posts", "BASE TABLE", "InnoDB", 42],
          ["blog", "comments", "BASE TABLE", "InnoDB", 120], ["blog", "v_post_stats", "VIEW", null, null],
          ["shop_demo", "orders", "BASE TABLE", "InnoDB", 80], ["shop_demo", "products", "BASE TABLE", "InnoDB", 24],
          ["shop_demo", "v_order_summary", "VIEW", null, null],
        ],
      },
      COLUMNS: {
        cols: ["TABLE_SCHEMA", "TABLE_NAME", "COLUMN_NAME", "DATA_TYPE", "IS_NULLABLE"],
        rows: [
          ["toolbox", "tasks", "id", "int", "NO"], ["toolbox", "tasks", "name", "varchar", "NO"],
          ["toolbox", "tasks", "last_status", "varchar", "YES"], ["blog", "posts", "title", "varchar", "NO"],
          ["blog", "posts", "published_at", "datetime", "YES"], ["blog", "comments", "body", "text", "NO"],
          ["shop_demo", "orders", "amount", "decimal", "NO"], ["shop_demo", "products", "stock", "int", "NO"],
        ],
      },
      SCHEMATA: {
        cols: ["SCHEMA_NAME", "DEFAULT_CHARACTER_SET_NAME"],
        rows: [["toolbox", "utf8mb4"], ["blog", "utf8mb4"], ["shop_demo", "utf8mb4"], ["information_schema", "utf8mb3"]],
      },
    },
  },
};

const PG_DEMO_DATA = {
  users: {
    cols: ["id", "name", "email", "created_at"],
    rows: Array.from({ length: 24 }, (_, i) => [i + 1, ["张伟", "王芳", "李娜", "刘洋"][i % 4] + (i >= 4 ? ` ${Math.floor(i / 4) + 1}` : ""), `user${i + 1}@example.com`, `2026-0${(i % 9) + 1}-1${i % 10} 09:00:00+08`]),
  },
  events: {
    cols: ["id", "user_id", "kind", "payload", "at"],
    rows: Array.from({ length: 200 }, (_, i) => [i + 1, (i % 24) + 1, ["translate", "download", "task_run", "login"][i % 4], `{"ok":${i % 5 !== 0}}`, `2026-11-${String((i % 20) + 1).padStart(2, "0")} 12:${String(i % 60).padStart(2, "0")}:00+08`]),
  },
  v_daily_events: {
    cols: ["day", "count"],
    rows: Array.from({ length: 14 }, (_, i) => [`2026-11-${String(i + 1).padStart(2, "0")}`, 40 + ((i * 17) % 60)]),
  },
};
const PG_DEMO_SCHEMA = [
  { name: "users", kind: "T", cols: colsOf([["id", "SERIAL", true], ["name", "VARCHAR(60)"], ["email", "VARCHAR(120)"], ["created_at", "TIMESTAMPTZ"]]) },
  { name: "events", kind: "T", cols: colsOf([["id", "BIGSERIAL", true], ["user_id", "INT"], ["kind", "VARCHAR(40)"], ["payload", "JSONB"], ["at", "TIMESTAMPTZ"]]) },
  { name: "v_daily_events", kind: "V", cols: colsOf([["day", "DATE"], ["count", "BIGINT"]]) },
];
const PG_ANALYTICS_DATA = {
  metrics: {
    cols: ["id", "name", "value", "recorded_at"],
    rows: Array.from({ length: 120 }, (_, i) => [i + 1, ["translate_calls", "download_mb", "task_runs", "active_days"][i % 4], Number((10 + ((i * 29) % 500) + 0.25).toFixed(2)), `2026-11-${String((i % 20) + 1).padStart(2, "0")} 00:00:00+08`]),
  },
  dashboards: {
    cols: ["id", "title", "owner_id"],
    rows: [[1, "翻译用量", 1], [2, "下载流量", 1], [3, "任务健康度", 2], [4, "活跃概览", 2], [5, "凭据到期", 3], [6, "版本分布", 3]],
  },
  v_metric_latest: {
    cols: ["name", "value"],
    rows: [["translate_calls", 486], ["download_mb", 3912.5], ["task_runs", 128], ["active_days", 21]],
  },
};
const PG_ANALYTICS_SCHEMA = [
  { name: "metrics", kind: "T", cols: colsOf([["id", "SERIAL", true], ["name", "VARCHAR(80)"], ["value", "NUMERIC(12,2)"], ["recorded_at", "TIMESTAMPTZ"]]) },
  { name: "dashboards", kind: "T", cols: colsOf([["id", "SERIAL", true], ["title", "VARCHAR(80)"], ["owner_id", "INT"]]) },
  { name: "v_metric_latest", kind: "V", cols: colsOf([["name", "VARCHAR(80)"], ["value", "NUMERIC(12,2)"]]) },
];

const DB_SEEDS = {
  SQLite: { version: "3.45", order: ["toolbox.db"], dbs: { "toolbox.db": { schema: SQLITE_SCHEMA, data: APP_DATA } } },
  PostgreSQL: { version: "16.2", order: ["demo", "analytics"], dbs: { demo: { schema: PG_DEMO_SCHEMA, data: PG_DEMO_DATA }, analytics: { schema: PG_ANALYTICS_SCHEMA, data: PG_ANALYTICS_DATA } } },
  MySQL: { version: "8.0.36", order: ["toolbox", "blog", "shop_demo", "information_schema"], dbs: MYSQL_DBS },
};