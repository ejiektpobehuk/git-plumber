# Changelog

## [Unreleased]

## [0.1.3] - 2025-08-20

### Added

- Highlights for modified files
- Packfile extensions are shown in the tree
- Pack Index (.idx) file support

### Changed

- git tree now uses actual file structure from `.git`
- idle rendering doesn't cause CPU spikes
- Loose Objects category is permanently available
- refs folder is pinned as second

## [0.1.2] - 2025-08-11

### Added

- CLI `view` command to open loose objects and pack files
- Live updates when there is a change in `.git`
- Highlight for the live updates

### Changed

- `pack` CLI command is now part of `view`
- CLI now uses TUI widgets to view the file
- CI pipelines moved to GitLab due to GH Actions free tier bug

## [0.1.1] - 2025-07-18

### Added

- Packfile header deep dive
- Pack object compression & header deep dive
- Loose file header view
- Minimal loose file preview

### Changed

- Loose file view in a repo tree

## [0.1.0] - 2025-06-10

### Added

- CLI help
- TUI
- Minimal support for pack files
- vim-like navigation
- initial CI with GitHub Actions

