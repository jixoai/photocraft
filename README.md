# FDE2E Photocraft

Rust / WebAssembly 图像编辑器，面向浏览器内的人机协作绘图与图像处理。**based on [PhotoCraft](https://github.com/storytold/photocraft)（MIT OR Apache-2.0）**，由 FDE2E 维护浏览器与 agent 增强版：

```text
同一 Rust 引擎与 egui 界面 ──▶ 原生桌面 / 浏览器 WebAssembly
命令注册表与 JSON Schema ──▶ CLI / MCP / WebMCP 共用命令契约
浏览器绘图与重计算       ──▶ worker 加速 / 兼容模式兜底
可编辑文档              ──▶ .pcraft 项目保存 / PNG 等格式导出
```

由 FDE2E（昭智羽名(厦门)科技有限公司）维护；对外品牌 **FDE2E Photocraft**，不使用上游品牌标识、社媒或社区入口。本项目不隶属于上游团队，也不代表上游官方发行版。

## Fork 链

- 上游：`storytold/photocraft`；本分支基于 `37a01ca87`，增强基线为 `agent-web-baseline-20261105`，改造与维护说明见 [FORK.md](FORK.md)。
- 许可：保留 [LICENSE-MIT](LICENSE-MIT)、[LICENSE-APACHE](LICENSE-APACHE)、[NOTICE](NOTICE) 与 [ATTRIBUTION.md](ATTRIBUTION.md)；上游版权、贡献记录和第三方许可不因更换品牌而删除。
- 本仓仍在 GitHub fork 网络内；默认分支 `agent-web` 是增强版，`main` 仅镜像上游。同步先在集成分支验证，未通过门禁不覆盖增强版。

## 开发

```bash
rustup target add wasm32-unknown-unknown
cargo run -p photocraft                      # 原生编辑器
cargo install trunk --locked                # 首次构建 Web 版需要
cd apps/photocraft-web && trunk serve --open # 浏览器开发
```

验证与发布：

```bash
bash scripts/downstream-check.sh quick
bash scripts/downstream-check.sh full
```

构建前提与 Web 打包见 [开发指南](docs/development.md)、[Web 发布说明](packaging/web/README.md)。原生入口保留，不将项目描述为已完成的纯 Web 重写。

## Agent 操作

- [WebMCP](docs/webmcp.md)：六个传输工具、命令发现、执行、检查、导入与导出。
- [控制协议](docs/control-protocol.md)：可编程操作与画面检查。
- [上游同步](docs/downstream-maintenance.md)：分支策略、回归门禁与契约基线。

当前基线有 964 个引擎命令，其中 135 个契约已完整审计、829 个明确标记为部分契约；不代表所有命令都已具备完整参数或输出契约。绘图工作流与构图策略属于 agent / skill，不能替代浏览器画面、项目保存和 PNG 导出的实际验收。

问题与功能请求请使用[本仓库 Issues](https://github.com/jixoai/photocraft/issues)。
