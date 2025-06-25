# git-plumber

> [!IMPORTANT]
> Pre-release software. Expect bugs and incomplete features.

[![asciicast](https://raw.githubusercontent.com/ejiektpobehuk/git-plumber/preview-assets/git-plumber.gif)](https://asciinema.org/a/724583)

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

### Development version

Dev versions are built from the latest `main` commit.
They are available:

- Nix Flake: `nix run github:ejiektpobehuk/git-plumber`
- Container images
  - `ejiek/git-plumber:dev` at Docker Hub
  - `ghcr.io/ejiektpobehuk/git-plumber:dev`
- [Arch User Repository](https://aur.archlinux.org/packages/git-plumber-git)
- [From source](#building-from-source)

### Versioned releases
They are built from tags, are more rare and still not stable.
However, they have better availability:

- [crates.io/git-plumber](https://crates.io/crates/git-plumber)
- Nix Flake: `nix run github:ejiektpobehuk/git-plumber/${VERSION}`
- Container images at
  - [Docker Hub](https://hub.docker.com/r/ejiek/git-plumber)
  - [GitHub Container Registry](https://github.com/ejiektpobehuk/git-plumber/pkgs/container/git-plumber)
- [Release Binaries](https://github.com/ejiektpobehuk/git-plumber/releases)

More packaging details are available in [issue #1](https://github.com/ejiektpobehuk/git-plumber/issues/1)

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
