# Domain Docs

工程类技能在探查代码库前，按以下规则消费本仓库的领域文档。

## 探查前先读

- 根目录 **`GLOSSARY.md`**；若存在根目录 **`GLOSSARY-MAP.md`**，则按它指向的各上下文 `GLOSSARY.md` 分别读取相关部分。
- **`docs/adr/`**：读涉及当前改动区域的 ADR。多上下文仓库还需检查 `src/<context>/docs/adr/`。

上述文件缺失时**静默跳过**：不提示缺失，不主动建议创建。`/domain-modeling`（经 `/grill-with-docs`、`/improve-codebase-architecture` 触达）会在术语或决策真正落定时按需生成。

## 文件结构

单上下文仓库（本仓库采用）：

```
/
├── GLOSSARY.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

多上下文仓库（根目录存在 `GLOSSARY-MAP.md`）：

```
/
├── GLOSSARY-MAP.md
├── docs/adr/                          ← 系统级决策
└── src/
    ├── ordering/
    │   ├── GLOSSARY.md
    │   └── docs/adr/                  ← 上下文级决策
    └── billing/
        ├── GLOSSARY.md
        └── docs/adr/
```

## 使用术语表中的词汇

输出中出现的领域概念（issue 标题、重构提案、假设、测试名）一律采用 `GLOSSARY.md` 的定义，不漂移到它明确回避的同义词。

需要的概念尚未收录时：要么是在自造项目不使用的语言（重新考虑），要么是真实缺口（记下留给 `/domain-modeling`）。

## 标记 ADR 冲突

输出与既有 ADR 矛盾时显式指出，而不是静默覆盖：

> _与 ADR-0007（event-sourced orders）冲突，但因……值得重新讨论。_
