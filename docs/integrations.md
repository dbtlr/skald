# Integrations (Experimental)

> **Note:** The `integrations` command is behind a feature flag. Install with `cargo install skald-cli --features integrations` to enable it.

The `sk integrations` command outputs config snippets for connecting Skald to other tools. Each snippet is printed to stdout so you can pipe or redirect it directly into your config.

## Usage

```sh
sk integrations              # List available integrations
sk integrations worktrunk    # Print worktrunk config snippet
sk integrations lazygit      # Print lazygit config snippet
sk integrations fugitive     # Print vim-fugitive config snippet
sk integrations hook         # Print git hook script
sk integrations hook --install          # Install hook into .git/hooks/
sk integrations hook --install --force  # Overwrite an existing hook
```

## Available Integrations

| Integration | What it does |
|-------------|--------------|
| `worktrunk` | Registers `sk` as the commit message generator in Worktrunk |
| `lazygit` | Adds a custom `C` keybinding that runs `sk commit` |
| `fugitive` | Adds a `<leader>sc` mapping that runs `sk commit -y` |
| `hook` | Installs a `prepare-commit-msg` hook that pre-fills commit messages |

---

## Worktrunk

[Worktrunk](https://github.com/dbtlr/worktrunk) is a CLI for managing git worktrees. It can call an external tool to generate commit messages when you commit from a worktree.

### Setup

Add the snippet to your Worktrunk user config at `~/.config/worktrunk/config.toml`:

```sh
sk integrations worktrunk >> ~/.config/worktrunk/config.toml
```

Or print it and add it manually:

```sh
sk integrations worktrunk
```

Output:

```toml
[tools.skald]
command = "sk"
args = ["commit", "--dry-run", "-y"]
```

---

## Lazygit

[Lazygit](https://github.com/jesseduffield/lazygit) supports custom commands that run in a subprocess. This snippet adds `C` as a keybinding in the files panel that launches `sk commit` interactively.

### Setup

Add the snippet to your lazygit config at `~/.config/lazygit/config.yml`:

```sh
sk integrations lazygit >> ~/.config/lazygit/config.yml
```

Or print it and add it manually:

```sh
sk integrations lazygit
```

Output:

```yaml
customCommands:
  - key: "C"
    context: "files"
    command: "sk commit"
    description: "AI-generated commit message"
    subprocess: true
```

The `subprocess: true` flag hands the terminal over to `sk commit` so the interactive carousel works correctly.

---

## Fugitive

[Vim-fugitive](https://github.com/tpope/vim-fugitive) is the standard Vim/Neovim git plugin. This snippet maps `<leader>sc` to run `sk commit -y` without leaving your editor.

### Setup

Add the snippet to your `~/.vimrc` or `~/.config/nvim/init.vim`:

```sh
sk integrations fugitive >> ~/.vimrc
```

Or print it and add it manually:

```sh
sk integrations fugitive
```

Output:

```vim
" Skald: AI-generated commit message
nnoremap <leader>sc :!sk commit -y<CR>
```

---

## Git Hook

The `hook` integration installs a `prepare-commit-msg` hook that automatically pre-fills the commit message buffer with an AI-generated message when you run `git commit`. The hook only runs on fresh commits — it skips merge commits, fixups, and any commit that already has a message.

### Print the Script

```sh
sk integrations hook
```

Output:

```sh
#!/bin/sh
# Skald prepare-commit-msg hook
COMMIT_MSG_FILE=$1
COMMIT_SOURCE=$2
if [ -z "$COMMIT_SOURCE" ]; then
    MSG=$(sk commit --dry-run -y 2>/dev/null)
    if [ $? -eq 0 ] && [ -n "$MSG" ]; then
        echo "$MSG" > "$COMMIT_MSG_FILE"
    fi
fi
```

### Install the Hook

Use `--install` to write and activate the hook automatically:

```sh
sk integrations hook --install
```

This writes the script to `.git/hooks/prepare-commit-msg` and sets it executable (`chmod 755`). The command must be run from the root of a git repository.

If a hook already exists, the command exits with an error and a suggestion to use `--force`:

```
error: hook already exists at .git/hooks/prepare-commit-msg
hint: use --force to overwrite the existing hook
```

### Overwrite an Existing Hook

```sh
sk integrations hook --install --force
```

### Worktrees

When run from a git worktree, `--install` resolves the correct hooks directory automatically. It follows the `gitdir:` pointer in the worktree's `.git` file to find the hooks directory.

### Piping the Script Manually

To install the hook without using `--install`:

```sh
sk integrations hook > .git/hooks/prepare-commit-msg
chmod +x .git/hooks/prepare-commit-msg
```
