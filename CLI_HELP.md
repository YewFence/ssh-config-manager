# Command-Line Help for `sshm`

This document contains the help content for the `sshm` command-line program.

**Command Overview:**

* [`sshm`↴](#sshm)
* [`sshm ls`↴](#sshm-ls)
* [`sshm clone`↴](#sshm-clone)
* [`sshm export`↴](#sshm-export)
* [`sshm import`↴](#sshm-import)
* [`sshm create`↴](#sshm-create)
* [`sshm edit`↴](#sshm-edit)
* [`sshm delete`↴](#sshm-delete)
* [`sshm prune`↴](#sshm-prune)
* [`sshm open`↴](#sshm-open)
* [`sshm open config`↴](#sshm-open-config)

## `sshm`

SSH config manager

**Usage:** `sshm <COMMAND>`

###### **Subcommands:**

* `ls` — List all SSH hosts
* `clone` — Clone an existing SSH host
* `export` — Export SSH config and public keys into a backup archive
* `import` — Import SSH config and public keys from a backup archive
* `create` — Create a new SSH host
* `edit` — Edit an existing SSH host
* `delete` — Delete an SSH host
* `prune` — Scan for unused key files in ~/.ssh
* `open` — Open ~/.ssh directory in system file manager



## `sshm ls`

List all SSH hosts

Reads ~/.ssh/config. No files are written or modified.

**Usage:** `sshm ls [OPTIONS]`

###### **Options:**

* `-s`, `--show` — Show full hostnames (default: masked)



## `sshm clone`

Clone an existing SSH host

Reads and writes ~/.ssh/config only. No other files are accessed.

**Usage:** `sshm clone <SOURCE> [NAME]`

###### **Arguments:**

* `<SOURCE>` — Source host alias to clone from
* `<NAME>` — New host alias name (prompted if omitted)



## `sshm export`

Export SSH config and public keys into a backup archive

Reads ~/.ssh/config and top-level ~/.ssh/*.pub files, then writes a local .zip archive. For backup/migration only, not sync. You handle copying, syncing, or encrypting the archive yourself. No private keys are read. No network requests are made.

**Usage:** `sshm export [OUTPUT]`

###### **Arguments:**

* `<OUTPUT>` — Output archive path (default: ./sshm-backup-YYYYMMDD-HHMMSS.zip)



## `sshm import`

Import SSH config and public keys from a backup archive

Reads a local .zip archive, validates it, then restores ~/.ssh/config and matching top-level ~/.ssh/*.pub files. Existing files that will be overwritten are backed up first. The archive does not include private keys; the matching private keys must already exist on this machine. No private keys are read. No network requests are made.

**Usage:** `sshm import [OPTIONS] <ARCHIVE>`

###### **Arguments:**

* `<ARCHIVE>` — Backup archive path

###### **Options:**

* `-y`, `--yes` — Skip the confirmation prompt



## `sshm create`

Create a new SSH host

Reads and writes ~/.ssh/config. If public key content is pasted as the identity file, it is also written to ~/.ssh/<name>.pub. No network requests are made.

**Usage:** `sshm create [OPTIONS] [NAME]`

###### **Arguments:**

* `<NAME>` — Host alias name (prompted if omitted)

###### **Options:**

* `-H`, `--hostname <HOSTNAME>` — HostName or IP address
* `-u`, `--user <USER>` — SSH user
* `-p`, `--port <PORT>` — SSH port
* `-i`, `--identity-file <IDENTITY_FILE>` — Path to identity file
* `-J`, `--proxy-jump <PROXY_JUMP>` — ProxyJump host
* `-d`, `--description <DESCRIPTION>` — Host description (written as a comment in config)



## `sshm edit`

Edit an existing SSH host

Reads and writes ~/.ssh/config. If public key content is pasted as the identity file, it is also written to ~/.ssh/<name>.pub. No network requests are made.

**Usage:** `sshm edit [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Host alias name to edit

###### **Options:**

* `-H`, `--hostname <HOSTNAME>` — HostName or IP address
* `-u`, `--user <USER>` — SSH user
* `-p`, `--port <PORT>` — SSH port
* `-i`, `--identity-file <IDENTITY_FILE>` — Path to identity file
* `-J`, `--proxy-jump <PROXY_JUMP>` — ProxyJump host
* `-d`, `--description <DESCRIPTION>` — Host description (written as a comment in config)



## `sshm delete`

Delete an SSH host

Reads and writes ~/.ssh/config only. Associated key files are not deleted.

**Usage:** `sshm delete <NAME>`

###### **Arguments:**

* `<NAME>` — Host alias name to delete



## `sshm prune`

Scan for unused key files in ~/.ssh

Reads ~/.ssh/config and scans the ~/.ssh/ directory listing. Read-only — no files are deleted or modified.

**Usage:** `sshm prune`



## `sshm open`

Open ~/.ssh directory in system file manager

Delegates to the system file manager (Explorer / Finder / xdg-open) or falls back to a subshell. sshm itself does not read any file contents.

**Usage:** `sshm open [COMMAND]`

###### **Subcommands:**

* `config` — Open ~/.ssh/config with default editor



## `sshm open config`

Open ~/.ssh/config with default editor

**Usage:** `sshm open config`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
