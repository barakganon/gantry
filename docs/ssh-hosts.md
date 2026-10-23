# Using Gantry with SSH hosts

Gantry can run Claude sessions on another machine over SSH and show them in the same sidebar as your local ones, with
the same status colors, alerts, search and recent sessions. Sessions keep running on the remote when you close the
window or lose the connection.

## What you need

- **On this machine:** the `ssh` command, and a way to log in to the host (a key, an `~/.ssh/config` alias, or a
  password).
- **On the remote:** Linux x86_64, and Claude Code installed and logged in for the remote user.
- **A `gantryd` on the remote.** This is a small headless daemon that owns the remote sessions. It needs no window or
  sound libraries. The Add SSH host wizard uploads it for you, as long as you run Gantry on Linux x86_64. From a
  Mac you have to install `gantryd` on the remote by hand.

## Add a host

1. Click "Add SSH host…" in the SSH hosts card in the sidebar, or in Settings > SSH hosts. While you have no hosts
   these are the only way in.
2. **Host:** pick an alias from your `~/.ssh/config`, or type `user@host`. Give the host a name; this is the name used
   in the sidebar and in the launcher. Under Advanced you can set:
   - **Remote command:** where `gantryd` lives on the remote. The default is `~/.local/bin/gantryd`.
   - **Port:** only if ssh's own configuration doesn't already give the right one.
   - **Authentication:** Automatic (default), Password, or Key. Automatic lets ssh decide. Password and Key force that
     method.
3. **Check:** Gantry connects once and looks at the remote. It reports whether the remote is Linux x86_64, whether
   `gantryd` is missing, older, the same or newer than this Gantry, and whether lingering is on (see below).
4. **Install:** if `gantryd` is missing or older, Gantry uploads the one that sits next to the running `gantry`. The
   remote moves it into place only after the byte count (and the sha256, where `sha256sum` exists) match, then runs
   `--version` to verify. If the remote is missing a library, the error lists every one of them with a single
   `apt install` line.
5. **Finish:** Gantry offers to run `loginctl enable-linger` for you, then saves the host and connects.

Cancel at any step saves nothing. Every step shares one ssh connection, so a password is asked only once.

If your `command` is not a plain path (`~/…` or `/…`, no spaces or shell syntax), Gantry cannot upload to it; install
`gantryd` there by hand.

### Keep sessions alive: lingering

The remote daemon's socket lives in the remote user's runtime directory. When the last ssh login of that user ends,
systemd can remove it and your sessions with it. Turn on lingering once on the remote, and the sessions survive:

```sh
loginctl enable-linger <user>
```

The wizard offers to do this. If it fails, it stays on Finish and shows the command, with a "Save and connect anyway"
button.

## Use a host

- **Start a session:** open the launcher (`Ctrl+Shift+O`). Folders from every host are in one list, with the host as a
  tag on remote rows. You can also type a path with a host prefix, such as `devbox:~/code/app`, and press Enter.
  Hosts that aren't connected yet are shown dashed; opening a folder on one connects it first.
- **Connect on demand:** a host connects the first time you use it. A click on its row in the SSH hosts card connects
  it too.
- **Sign-in questions:** passwords, key passphrases and host-key confirmations appear as dialogs in Gantry. They are
  never stored or logged.
- **What remote sessions look like:** an SSH icon on their rows, in the title-bar chip and in pane headers. A remote
  project heading reads `<host> - <project>`, and the same folder on two hosts is two projects.
- **Remote sessions use plain `claude`.** They use the remote user's own config dir. Your local config dirs, commands
  and project colors don't apply on a remote.
- **Per-host features:** take over, conversation search, recent sessions and the launcher's folder list all work for
  each connected host.

## If the connection drops

Gantry retries after 1, 2, 5 and 10 seconds without asking for anything. If a retry needs a password it stops with
"Needs sign-in"; after that it shows a Reconnect button. Every pane of a host that isn't usable shows a card that says
why. The sessions on the remote keep running meanwhile (given lingering).

An attempt fails if the handshake doesn't finish within 30 seconds. The Connecting card has a Cancel button.

If another Gantry takes over the host's daemon, you see "opened elsewhere" with a Reconnect button. It is never
retried by itself, because two windows would fight over it.

## Disconnect

The × on a connected host's row disconnects it. If nothing is running there, the daemon stops at once. If sessions are
running you can Detach (leave them running), Close all sessions, or Cancel. Quitting Gantry with "Stop all sessions"
stops only this machine's sessions; SSH hosts are disconnected and keep running.

## Update a host

When a host's `gantryd` is older than your Gantry, its row shows a `↑ <version>` pill. Click it to upload the new
version and restart the remote daemon. Restarting ends any sessions running there, so Gantry asks first when there are
some.

## Manage hosts

Settings > SSH hosts shows each host's state and lets you:

- **Rename** it. Its panes follow. A rename onto a name that still has panes in the layout is refused.
- **Edit the target.** On a connected host this applies after you reconnect.
- **Change the authentication method.**
- **Remove** it. Gantry disconnects, drops its panes after a confirm, and leaves the remote untouched.

## Troubleshooting

- **"Not installed" or "Update needed" card:** use its Install… or "Update and restart" button.
- **"Not listed" card:** the pane belongs to a host that is no longer in your settings. Add it again or close the pane.
- **Sessions vanished after logging out of the remote:** lingering is off. Run `loginctl enable-linger <user>` there.
- **Password asked on every connect:** that's normal for password auth; use a key or an `~/.ssh/config` entry with an
  agent if you want to avoid it.
- **Connection never completes:** try `ssh <target>` in a terminal first. If that doesn't log in cleanly, Gantry can't
  either.
