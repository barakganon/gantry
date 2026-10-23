# Using more than one Claude config dir

Claude Code keeps its settings, login and session history in a config dir (`~/.claude` by default). If you use Claude
with more than one account, for example personal and work, each account gets its own dir. Gantry can run sessions from
all of them side by side, each with its own badge and color in the sidebar.

## How Claude picks a config dir

Claude reads the `CLAUDE_CONFIG_DIR` environment variable. Without it, it uses `~/.claude`. So outside Gantry, a second
account is just a command with the variable set:

```sh
CLAUDE_CONFIG_DIR=~/.claude-work claude
```

The first run in a new dir asks you to log in, and that login stays in that dir.

## In Gantry you don't set the variable yourself

You register each dir once, and Gantry sets `CLAUDE_CONFIG_DIR` for every session it starts in that dir (new sessions,
Continue and resume). A session always resumes in the dir it was recorded in.

### 1. Create the dir (once)

Open a terminal, either your system terminal or a terminal pane in Gantry, and run this command there. It starts Claude
with the variable set for that one run. Log in when Claude asks:

```sh
CLAUDE_CONFIG_DIR=~/.claude-work claude
```

### 2. Add it to Gantry

- **First start:** the setup wizard lists the config dirs it finds (`$CLAUDE_CONFIG_DIR`, `~/.claude`, and siblings
  such as `~/.claude-work` or `~/.config/claude*`). Tick the ones you want, give each a name, and pick a default.
- **Later:** Settings > Sessions > "Detect Claude dirs…" opens the same list for dirs you have not added yet.

The default dir is the one used when nothing else decides (see below). With no dirs added, Gantry uses one implicit
"Default" dir.

### 3. Choose a dir when you start a session

A new Claude session uses, in this order:

1. The dir you pick: the badge in the launcher (`Ctrl+Shift+O`; Tab cycles it) or the "New Claude here with" submenu
   in the right-click menu.
2. The dir remembered for exactly that folder. An explicit pick is remembered (`folderClaudeDirs`).
3. The default dir.

Badges (the first letter of the dir's name on its color) appear in the sidebar, title-bar chip, pane headers and recent
sessions once you have two or more dirs.

## Good to know

- Gantry only reads config dirs. It never writes to them.
- Search, recent sessions and take over cover every registered dir.
- SSH hosts always use the remote's plain `claude` and its own dir. Your local dirs and `command` do not apply there.
- Dev containers run `claude` with the container's own config dir. See [dev-containers.md](dev-containers.md) for how to
  mount a dir into one.
