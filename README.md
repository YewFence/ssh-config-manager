# sshm

SSH config manager — a CLI tool for managing `~/.ssh/config`.

**sshm runs fully offline** — it operates only on files under `~/.ssh/` and makes no network requests of any kind.

---

## Installation

### Homebrew (macOS / Linux)

```bash
brew tap YewFence/tap
brew install sshm
```

### cargo

```bash
cargo install sshm
```

### Pre-built binaries

Download from [Releases](https://github.com/YewFence/ssh-config-manager/releases).

---

## Quick Start

```bash
# List all hosts
sshm ls

# Create a host (interactive)
sshm create myserver
sshm create myserver --hostname 192.168.1.10 --user root

# Edit a host (interactive)
sshm edit myserver
sshm edit myserver --user ubuntu   # directly update single field

# Delete a host
sshm delete myserver
```

---

## Commands

### `sshm ls`

List all SSH hosts. Hostnames are masked by default.

```bash
sshm ls
sshm ls --show   # reveal full hostnames
```

output:

```bash
+------------+-----------------+----------+-------+---------------------+------------+
| NAME       | HOSTNAME        | USER     | PORT  | IDENTITY FILE       | PROXY JUMP |
+============================================================================+========+
| myserver   | exa***.com      | admin    | 22    | ~/.ssh/id_ed25519   | -          |
|------------+-----------------+----------+-------+---------------------+------------|
| devbox     | 192***.100      | ubuntu   | 2222  | -                   | jum***ost  |
+------------+-----------------+----------+-------+---------------------+------------+
```

### `sshm create [name]`

Create a new host. If flags are provided, prompts are skipped for those fields.

```bash
# Interactive
sshm create
sshm create myserver

# Non-interactive (all fields via flags)
sshm create myserver -H 192.168.1.10 -u root -p 2222 -i ~/.ssh/id_ed25519
```

**Flags:** `-H/--hostname`, `-u/--user`, `-p/--port`, `-i/--identity-file`, `-J/--proxy-jump`, `-d/--description`

### `sshm edit <name>`

Edit an existing host. Flags update fields directly; omitted fields prompt interactively with current values as defaults.

```bash
# Interactive edit
sshm edit myserver

# Direct update (no prompts)
sshm edit myserver --user ubuntu
sshm edit myserver -H newhost.example.com -p 2222
```

**Flags:** same as `create`

### `sshm delete <name>`

Delete a host (prompts for confirmation).

```bash
sshm delete myserver
```

### `sshm clone <source> [name]`

Clone an existing host configuration.

```bash
sshm clone myserver myserver-backup
```

### `sshm prune`

List unreferenced key files in `~/.ssh/` (read-only, no files deleted).

```bash
sshm prune
```

### `sshm open`

Open `~/.ssh/` in system file manager.

```bash
sshm open           # open directory
sshm open config    # open config in editor
```

---

## IdentityFile Input Formats

The `--identity-file` / `-i` flag accepts three formats:

| Format | Example | Result |
|--------|---------|--------|
| Full path | `~/.ssh/id_ed25519` | Written as-is |
| Filename only | `id_ed25519` | Expanded to `~/.ssh/id_ed25519` |
| Public key content | `ssh-ed25519 AAAA...` | Prompts for filename, saved to `~/.ssh/<name>.pub` |

---

## Full CLI Reference

For complete command-line documentation (all subcommands, flags, and options), see **[CLI_HELP.md](./CLI_HELP.md)**.

This file is auto-generated from source code and always up-to-date.

---

## Security

sshm is fully offline — it makes no network requests of any kind.

| Command | File access |
|---------|-------------|
| `ls`, `edit`, `delete`, `clone` | Read `~/.ssh/config` |
| `create`, `edit` | Read + write `~/.ssh/config` |
| `create`, `edit` (public key paste) | Also writes `~/.ssh/<name>.pub` |
| `prune` | Read-only scan of `~/.ssh/` |
| `open` | Delegates to system file manager |

sshm never reads private key material.

---

## Notes

- Top-level comments and unrecognized directives (e.g. `ForwardAgent`) are preserved when editing
- File permissions are automatically set to `600` after writing on Unix systems
