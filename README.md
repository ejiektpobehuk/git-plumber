# git-plumber

[![asciicast](https://asciinema.org/a/yekhZM8XzNzAq6IAzLuRUBzsr.svg)](https://asciinema.org/a/yekhZM8XzNzAq6IAzLuRUBzsr)

> _🎥 Click above to watch git-plumber in action (asciinema demo)_

> **Explore a `.git/` directory and peek into git’s internals from a terminal.**
> _A visual, interactive companion to “Pro Git” Chapter 10 and anyone curious about what’s under git’s hood._

---

## What is _git-plumber_?

[git-plumber](https://github.com/ejiektpobehuk/git-plumber) is a CLI and TUI application for **exploring the internals of git repositories** in a safe, read-only way.
Browse and understand the contents of the `.git/` directory: refs, trees, blobs and more.
Demystifying compressed and binary “plumbing” beneath git’s familiar porcelain appearance.

Perfect for learning, live experimentation, or just satisfying your curiosity.

### Use cases


- **Learning with _[“Pro Git” 10. Git Internals](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain))_**: Fire up `git-plumber` in a test repository and see immediately how every command reshapes your repo’s internals.
- **Understanding git storage**: See for yourself how git stores differences between versions — the reason this app was created!

### What _git-plumber_ is not?

- **Not** a replacement for everyday git workflow
- **Not** a “porcelain” UI like [gitui](https://github.com/extrawurst/gitui) or [lazygit](https://github.com/jesseduffield/lazygit)
- **Not** an interface for running plumbing commands, but a worthy alternative to `git cat-file` or `git verify-pack`

---

## Installation

> [!WARNING]
> Early version - best to build from source

_Packaging is coming soon! Planned:_

- crates.io
- Nix Flake
- Docker
- Linux distros

### Building from source

Prerequisites:
- [Rust and Cargo](https://rustup.rs/) installed


```bash
git clone https://github.com/ejiektpobehuk/git-plumber.git
cd git-plumber
cargo install --path .
```

---

## Roadmap & Contributions

This app is my git learning project.
It's going to be more complete as my knowledge grows.

For planned features checkout [issues at GitHub](https://github.com/ejiektpobehuk/git-plumber/issues).

**Major goals**:

- Navigation hints
- Support for all native git files
- Clear "unsupported" indicators
- Internationalization (i18n)
- [git-bug](https://github.com/git-bug/git-bug) support
- [jj](https://github.com/jj-vcs/jj) support

**PRs/issues welcome — no need to be a git guru!**
Beginners and documentation helpers are especially appreciated.
If something is unclear, that’s a sign the app can get better: lease [open an issue](https://github.com/ejiektpobehuk/git-plumber/issues/new) or start a discussion!

---

## License

MIT

---

## Credits & Inspiration

- *Pro Git*, [Chapter 10: Git Internals](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain)
- [Git pack-format documentation](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain)

---

**Not a git management tool.
Not for your day-to-day workflow.
This is for those who want to see git’s wiring and learn how it all fits together.**

🕳️ *Happy plumbing!* 🔧
