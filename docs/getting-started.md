# Getting Started

## Install

### From source (cargo)

```sh
cargo install --path crates/skald
```

This installs the `sk` binary to your Cargo bin directory.

### Verify

```sh
sk --version
```

## First Run

Run `sk --help` to see all available commands:

```sh
sk --help
```

## Shell Completions

Generate completions for your shell and add them to your shell config:

```sh
# zsh
sk completions zsh > ~/.zfunc/_sk

# bash
sk completions bash > ~/.local/share/bash-completion/completions/sk

# fish
sk completions fish > ~/.config/fish/completions/sk.fish
```

## What's Next

Most commands are stubs in this release. The commit workflow arrives in M1, configuration in M1/M2, and PR workflows in M2.
