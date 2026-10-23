# Using Gantry with VS Code dev containers

Gantry lists Claude sessions that run in a dev container under their host folder, starts new ones inside the container,
and resumes them there. This works when the container can see your Claude config dir, so that the sessions Claude writes
inside the container land in a dir Gantry reads on the host.

## What Gantry needs

1. **Docker.** Gantry uses the `docker` CLI, only on demand (at start, before a launch, when it meets an unknown
   container). It never reads a container's environment.
2. **A VS Code dev container.** VS Code labels its containers with `devcontainer.local_folder`; that label links the
   container to the project folder on the host.
3. **Your host Claude config dir mounted into the container**, at the same path or at the container user's home.
4. **`CLAUDE_CONFIG_DIR` set for the container** through `remoteEnv`, pointing at the mounted dir. Gantry reads it from
   the container's metadata and passes it to the sessions it starts there.
5. **Claude installed in the container.**

## A working `devcontainer.json`

This is the shape used across several real projects. Adjust the user name and paths.

```json
{
  "name": "my-project",
  "dockerFile": "../Dockerfile",
  "remoteUser": "me",
  "remoteEnv": { "CLAUDE_CONFIG_DIR": "/home/me/.claude" },
  "features": {
    "ghcr.io/devcontainers/features/common-utils:2": {
      "username": "me",
      "uid": "1000",
      "gid": "1000",
      "installZsh": false,
      "upgradePackages": false
    }
  },
  "mounts": [
    "source=/home/me/.claude,target=/home/me/.claude,type=bind,consistency=cached"
  ],
  "postCreateCommand": "curl -fsSL https://claude.ai/install.sh | bash && echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc"
}
```

What each part does:

| Part | Why |
| --- | --- |
| `mounts` | Binds the host config dir into the container. Claude's login, settings and session transcripts live there, so the container shares them with the host and Gantry sees the sessions. |
| `remoteEnv.CLAUDE_CONFIG_DIR` | Points Claude at the mounted dir. With it set, Claude also keeps its `.claude.json` inside that dir, so it travels through the same mount and you do not mount `~/.claude.json` separately. |
| `remoteUser` + `common-utils` with `uid`/`gid` `1000` | Creates a non-root user whose ids match the host user's, so the bind-mounted files are readable and writable. Check yours with `id -u` and `id -g` on the host. |
| `postCreateCommand` | Installs Claude with the native installer and puts `~/.local/bin` on `PATH`. Gantry starts Claude through `bash -ic`, so `~/.bashrc` is read. |

Using the host path as the mount target keeps the host and container paths identical, which is the simplest setup.
`${localEnv:HOME}` works for the source if you share the file between machines:
`source=${localEnv:HOME}/.claude,target=/home/me/.claude,type=bind`.

### Running as root

A container without `remoteUser` runs as root with home `/root`. Mount to `/root/.claude` and set
`CLAUDE_CONFIG_DIR` to match:

```json
"remoteEnv": { "CLAUDE_CONFIG_DIR": "/root/.claude" },
"mounts": ["source=/home/me/.claude,target=/root/.claude,type=bind"]
```

Files Claude creates in the mount are then owned by root on the host. Prefer the non-root setup above.

### Another config dir

To use a dir other than `~/.claude` (see [claude-config-dirs.md](claude-config-dirs.md)), change the mount source and
keep `CLAUDE_CONFIG_DIR` pointing at the target:

```json
"remoteEnv": { "CLAUDE_CONFIG_DIR": "/home/me/.claude-work" },
"mounts": ["source=/home/me/.claude-work,target=/home/me/.claude-work,type=bind"]
```

The dir must also be registered in Gantry (`claudeDirs`), or its sessions are not indexed. Gantry does not add the
host-side dir's settings to a container session: the container's own `CLAUDE_CONFIG_DIR` applies.

## Using it in Gantry

1. Open the folder in VS Code and "Reopen in Container" once, so the container exists. Keep VS Code's
   `"shutdownAction": "none"` in `devcontainer.json` if you want the container to survive closing the VS Code window;
   otherwise VS Code stops it. Gantry starts a stopped container itself.
2. In Gantry, start a Claude session in that folder (New, `Ctrl+Shift+O`, split, or a new tab). Because the container
   mounts the folder, Gantry asks **Dev container** or **This machine**. Pick Dev container.
3. The session runs through `docker exec` inside the container, is listed under the host folder in the sidebar, and
   gets the usual status colors and alerts. Continue and resume reopen it in the container. Take over works for
   container sessions on Linux.

## Troubleshooting

- **No "Dev container" choice:** the container was not created by VS Code's dev containers (no
  `devcontainer.local_folder` label), or the folder is not mounted into it.
- **Claude asks to log in again inside the container:** the config dir is not mounted where `CLAUDE_CONFIG_DIR` points,
  or the mount source does not exist on the host (Docker then creates an empty directory there).
- **Permission denied on the mounted dir:** the container user's uid/gid differ from the host's. Match them as above.
- **`claude: command not found`:** the install did not finish or `~/.local/bin` is not on `PATH` in `~/.bashrc`.
- **Sessions run but are missing in the sidebar:** the host config dir that is mounted is not registered in Gantry, or
  it is not the dir Claude writes to inside the container.
- **Do not mount `~/.claude.json` as a separate file** when you set `CLAUDE_CONFIG_DIR`. If the file does not exist on
  the host, Docker creates a directory in its place and Claude breaks.
