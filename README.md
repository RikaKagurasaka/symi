<img src="./logo.svg" alt="Symi" width="200" height="200">

# Symi

Symi(**SY**nthesized **MI**crotone)是一种可用于微分音乐的标记语言，受simai语启发创作。

目前支持多种音高（唱名、倍音、平均律、音分、频率）、节奏（时长、量化）表示形式，支持反常拍号和任意有理拍。还支持宏定义和调用，简化重复段落和和声编写。

## Symi Editor

Symi Editor是一个基于Tauri的跨平台桌面应用程序，提供了一个用户友好的界面来编写、编辑和预览Symi代码。它支持实时语法高亮、错误检查、实时回放和钢琴窗预览。

# 文档

你可以在[文档站点](https://symi.rika.link/){target=_blank}找到Symi的使用说明和语法介绍。

# 开发和构建
1. 确保你已经安装了Rust和Node.js工具链。

2. 克隆仓库并安装依赖

  ```bash
  git clone https://github.com/RikaKagurasaka/symi.git
  cd symi/editor
  npm install
  ```

3. 启动开发服务器（可选）

  ```bash
    npm run tauri dev
  ```

4. 构建发布版本

  ```bash
    npm run tauri build
  ```

# 许可证

本项目采用 Apache 2.0 许可证，详情请参阅 [LICENSE](./LICENSE) 文件。
