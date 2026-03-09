# Symi Docs

`symi-docs` 是 Symi 项目的官方文档站，基于 `Nuxt 4 + Nuxt Content + Nuxt UI`，主要提供：

- Symi 语法与使用指南（中文）
- 更新日志与下载入口
- LLM 友好的文档导出（`/llms.txt`、`/llms-full.txt`）
- GitHub Release 信息与安装包下载代理 API

线上站点默认地址：<https://symi.rika.link>

---

## 技术栈

- Nuxt 4
- @nuxt/content 3
- @nuxt/ui 4
- @nuxt/image
- nuxt-llms
- Nitro（`cloudflare-module` preset）

---

## 环境变量

复制 `.env.example` 后按需填写：

```bash
cp .env.example .env
```

可用变量：

- `NUXT_PUBLIC_SITE_URL`：站点公网地址（用于 OG 等场景）
- `NUXT_PUBLIC_GITHUB_REPO`：GitHub 仓库，默认 `RikaKagurasaka/symi`
- `GITHUB_TOKEN`：可选；用于提升 GitHub API 访问稳定性（避免匿名限流）

---

## 安装依赖

```bash
bun install
```

---

## 本地开发

```bash
bun dev
```

- 本地开发端口：`http://localhost:3462`

---

## 构建与预览

```bash
bun run build
bun run preview
```

常用质量检查：

```bash
bun run lint
bun run typecheck
```

---

## 内容组织

内容位于 `content/`，通过 `content.config.ts` 定义了两个集合：

- `landing`：首页（`content/index.md`）
- `docs`：其余文档页面（排除 `index.md`）

当前文档结构示例：

```text
content/
  index.md
  1.getting-started/
  2.grammars/
  3.llm.md
  4.updating-log.md
```

---

## 关键路由

### 页面与原始内容

- `/`：文档首页
- `/<doc-path>`：文档详情页
- `/raw/<doc-path>.md`：将文档页面输出为 Markdown（用于机器消费/集成）

### LLM 路由

- `/llms.txt`
- `/llms-full.txt`

### Release API

- `/api/releases/latest`：读取最新发布信息（含资产列表）
- `/api/releases/assets/:assetId`：代理下载 GitHub Release 资产

---

## 如何新增文档

1. 在 `content/` 下新增 `.md` 文件（建议按现有编号目录组织）
2. 添加 frontmatter（至少 `title`、`description`）
3. 本地运行 `bun dev` 预览导航、目录与渲染效果
4. 提交前执行 `bun run lint && bun run typecheck`

---

## 部署说明

项目当前 Nitro 配置使用 `cloudflare-module`，并已启用：

- `nitro.experimental.wasm = true`（优化 Shiki/WASM 场景）
- 预渲染首页（`/`）并爬取链接

如切换部署平台，请先核对 `nuxt.config.ts` 中的 Nitro preset 与构建策略。

---

## License

Apache-2.0
