# sshm

SSH config manager — 用于管理 `~/.ssh/config` 的命令行工具。

## 安装

```bash
cargo install --path .
```

## 用法

### 列出所有 host

```bash
sshm ls
```

```
+------------+-----------------+----------+-------+---------------------+------------+
| NAME       | HOSTNAME        | USER     | PORT  | IDENTITY FILE       | PROXY JUMP |
+============================================================================+========+
| myserver   | example.com     | admin    | 22    | ~/.ssh/id_ed25519   | -          |
|------------+-----------------+----------+-------+---------------------+------------|
| devbox     | 192.168.1.100   | ubuntu   | 2222  | -                   | jumphost   |
+------------+-----------------+----------+-------+---------------------+------------+
```

### 创建 host

**交互式：**

```bash
sshm create
# 或简写
sshm c
```

**带参数（可省略部分交互）：**

```bash
sshm create myserver --hostname example.com --user admin
sshm create myserver -H example.com -u admin -p 2222
```

当 `name` 和 `--hostname` 同时提供时完全跳过交互。

**可用参数：**

| 参数 | 简写 | 说明 |
|------|------|------|
| `--hostname` | `-H` | 主机名或 IP |
| `--user` | `-u` | SSH 用户名 |
| `--port` | `-p` | 端口（默认 22）|
| `--identity-file` | `-i` | 密钥文件路径 |
| `--proxy-jump` | `-J` | 跳板机 host 别名 |

**IdentityFile 字段支持三种输入方式：**

- **完整路径**（如 `~/.ssh/id_ed25519`）→ 原样写入
- **纯文件名**（如 `id_ed25519`）→ 自动补全为 `~/.ssh/id_ed25519`
- **公钥内容**（粘贴 `ssh-ed25519 AAAA...`）→ 询问文件名，保存到 `~/.ssh/<name>.pub`

### 编辑 host

```bash
sshm edit myserver
# 或简写
sshm e myserver
```

现有字段会作为默认值预填，直接回车保持不变。

## 说明

- 读写 `~/.ssh/config`，不存储任何密钥内容
- 文件中手写的注释（顶部）和未识别的指令（如 `ForwardAgent`）会被保留
- Unix 系统下写入后自动设置文件权限为 `600`
