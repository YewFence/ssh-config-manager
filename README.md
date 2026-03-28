# sshm

SSH config manager — a CLI tool for managing `~/.ssh/config`.

**sshm runs fully offline** — it only reads and writes `~/.ssh/config` on your local machine and makes no network requests.

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

Download the latest binary for your platform from the [Releases](https://github.com/YewFence/ssh-config-manager/releases) page, then place it somewhere on your `$PATH`.

### Build from source

```bash
cargo install --path .
```

## Usage

### List hosts

Hostnames are masked by default (first and last 3 characters kept) to avoid leaking sensitive info:

```bash
sshm ls
```

```
+------------+-----------------+----------+-------+---------------------+------------+
| NAME       | HOSTNAME        | USER     | PORT  | IDENTITY FILE       | PROXY JUMP |
+============================================================================+========+
| myserver   | exa***.com      | admin    | 22    | ~/.ssh/id_ed25519   | -          |
|------------+-----------------+----------+-------+---------------------+------------|
| devbox     | 192***.100      | ubuntu   | 2222  | -                   | jum***ost  |
+------------+-----------------+----------+-------+---------------------+------------+
```

Use `--show` / `-s` to reveal full hostnames:

```bash
sshm ls --show
sshm ls -s
```

Or set `SSHM_SHOW=1` to make it permanent (accepts `1` / `true` / `yes`):

```bash
export SSHM_SHOW=1
sshm ls
```

### Create a host

**Interactive:**

```bash
sshm create
# alias
sshm c
```

**With flags (skips prompts for provided fields):**

```bash
sshm create myserver --hostname example.com --user admin
sshm create myserver -H example.com -u admin -p 2222
```

When both `name` and `--hostname` are provided, all prompts are skipped.

**Available flags:**

| Flag | Short | Description |
|------|-------|-------------|
| `--hostname` | `-H` | Hostname or IP address |
| `--user` | `-u` | SSH username |
| `--port` | `-p` | Port (default: 22) |
| `--identity-file` | `-i` | Path to private key |
| `--proxy-jump` | `-J` | ProxyJump host alias |

**`IdentityFile` accepts three input formats:**

- **Full path** (e.g. `~/.ssh/id_ed25519`) — written as-is
- **Filename only** (e.g. `id_ed25519`) — expanded to `~/.ssh/id_ed25519`
- **Public key content** (paste `ssh-ed25519 AAAA...`) — prompts for a filename and saves to `~/.ssh/<name>.pub`

### Edit a host

```bash
sshm edit myserver
# alias
sshm e myserver
```

Existing values are pre-filled as defaults. Press Enter to keep them unchanged.

### Delete a host

```bash
sshm delete myserver
# alias
sshm d myserver
```

Prompts for confirmation before deleting.

### Scan for unreferenced key files

```bash
sshm prune
```

Scans `~/.ssh/` and lists key files not referenced by any host entry. Read-only — no files are modified.

### Open the ~/.ssh directory

```bash
sshm open
```

Opens `~/.ssh/` in the system file manager (Explorer on Windows, Finder on macOS, `xdg-open` on Linux). Prints the path if no GUI is available.

## Notes

- Reads and writes `~/.ssh/config` only — no key material is ever stored or transmitted
- Top-level comments and unrecognized directives (e.g. `ForwardAgent`) in the config file are preserved
- File permissions are automatically set to `600` after writing on Unix systems
