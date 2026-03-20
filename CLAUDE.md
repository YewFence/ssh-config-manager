# sshm — CLAUDE.md

## 项目概览

管理 `~/.ssh/config` 的 CLI 工具。不存储任何密钥内容，只操作路径引用和配置字段。

## 文件结构

```
src/
├── main.rs               # 入口，CLI 路由
├── cli.rs                # Clap 结构体（Cli, Commands）
├── config/
│   ├── mod.rs            # load_config / save_config / ssh_config_path
│   ├── types.rs          # SshHost, SshConfig
│   ├── parser.rs         # &str -> SshConfig（逐行状态机）
│   └── writer.rs         # SshConfig -> String（全量重写）
└── commands/
    ├── mod.rs            # 共用工具：resolve_identity_file, sanitize_filename
    ├── ls.rs             # sshm ls
    ├── create.rs         # sshm create / c，含 CreateFlags, prompt_host
    └── edit.rs           # sshm edit / e，复用 create::prompt_host
```

## 关键设计

### SSH config 解析
- 自写 parser，不用第三方 crate（第三方 crate 面向连接，无法干净写回）
- `SshHost.extra` 保留所有未识别指令（`ForwardAgent` 等），写回时原样输出
- `SshConfig.header_comments` 保留文件顶部注释，写回时不丢失

### 写回策略
全量重写（`writer::serialize`），不做原地 patch。SSH config 文件通常很小，性能无问题。

### IdentityFile 处理（`commands/mod.rs::resolve_identity_file`）
三种输入类型自动识别：
1. **公钥内容**（以 `ssh-ed25519 ` 等开头）→ 询问文件名 → 写入 `~/.ssh/<name>.pub` → config 存路径
2. **纯文件名**（无 `/` 或 `\`）→ 打印提示 → config 存 `~/.ssh/<filename>`
3. **完整路径** → 原样写入

### 非交互模式（create）
`name` 和 `--hostname` 同时通过 flags 提供时，跳过交互直接写入。

## 依赖

| Crate | 用途 |
|-------|------|
| `clap 4` | CLI，derive 模式，子命令别名 |
| `inquire 0.7` | 交互提示，`.with_default()` 支持预填 |
| `anyhow 1` | 错误处理 |
| `comfy-table 7` | ls 表格渲染 |
| `dirs 5` | 跨平台 home 目录 |

## 常用命令

```bash
cargo build
cargo run -- ls
cargo run -- create
cargo run -- create <name> --hostname <host>
cargo run -- edit <name>
```
