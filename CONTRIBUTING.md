# Contributing to `git-plumber`

Thank you for your interest in improving `git-plumber`!
There are many ways to help the project!

## Use the app and provide feedback

I'm happy to hear from you, the user.
You can reach out publicly via [GitHub issues](https://github.com/ejiektpobehuk/git-plumber/issues) or privately via `git-plumber@ejiek.id` email.

I probably won't see your public feedback on social networks, but others might.
Spreading the word helps tremendously.

## Internationalisation and localisation (i18n)

I'd like to make the app available in multiple languages.
Unfortunately, `git-plubmer` is still early in the development cycle and I haven't implemented any support for i18n.
Currently, it's not a priority because the app is still in the experimental stage.
Architecture and data representation may change a lot.
Stay tuned though ^.~

## Packaging

Yes, please!

Currently, the app is only a binary built with Rust.
The package has to build it or pull it from the releases.
It's going to be a bit more sophisticated, for example, with shell completions.

General packaging recommendations are coming soon.

## Development

The app is in the exploratory stage.
Right now, the focus is on figuring out what the User Experience should be.
We can fix the implementation details later.

Current priorities:
1. Educational value;
1. Usability;
1. Performance.

First, we make a piece of information available, then we make it easy to access and consume, and only then do we make it fast to access.


We don't write to the repo under inspection.
`git-plumber` is a read-only application.
Avoid writing to `.git/` or working directory at all costs.
XDG might be a good way to save a file out of the repo if you absolutely have to.
If a write is ever required, make it explicit, gated, and well-documented.
Tour of git might be a good thing to implement writing for.

### Design

`git-plumber` is a TUI-first app with CLI catching up.

I prefer to rely on common navigation patterns for TUI.
The general rule is to make navigation intuitive for a regular user and support vim-motions.
This might mean having several key bindings to do the same thing.

I want navigation hints to be clean, not cluttered with all available keys.
Keys in hints are highlighted with a color to be distinguishable from the rest of the text.
Directional navigation hints (←↕→) are highlighted with a color as keys.
If some direction is unavailable, it's grayed out instead of hidden to make hints more consistent and less jittery.
There are several techniques that I use to make hints more compact:
- Icons instead of keys: `←↕→` instead of `hjkl and arrows`
- Highlighting a key inside the word: `(Q)uit` instead of `Q to quit`

The app should be sufficient and well understood without separate documentation.
TUI navigation should be intuitive and explained in hints or a help pop-up.
CLI should provide sufficient `--help` and user-friendly errors if an extra argument is required.
Git concepts should be well described inside the app.

Responsiveness.
Some operations in `git-plumber` are long by design.
They should provide a loading indicator and not slow down the UI.

Accessibility.
I don't know a thing about making a TUI application accessible, but I hope to figure out a way to make a CLI output available.
Currently, CLI output is not reader-friendly because of preusographics.
I don't want to lose preusographics because they help to understand the information.
So an alternative output format might be a good option.
It would be great to get some input from a person experienced in creating or using an accessible application.

### Architecture

Async. This app is UI heavy and might benefit from async, but I try to avoid it because I don't like the current state of async in Rust.

Avoid `unsafe`.
If absolutely necessary, isolate and document reasoning and invariants.
Clearly communicate in PR that it contains unsafe.

TUI architecture.
It is inspired by an ELM architecture.
One of the goals is to minimise redraws.
So there is no FPS target.
Redraws should happen only when there is a change.
We use signals to indicate a need for a redraw.
There is one exception where the FPS approach should be acknowledged - animations.
Change highlights are highly dynamic and spread over a period of time.
During animations, a frequent tick signal is used; however, it should be stopped when the animations are finished.

Do not panic in the TUI.
Prefer errors with context and user-friendly messages.
Handle partial/corrupt repositories and unsupported objects gracefully; clearly surface what is unsupported.

Make common operations non-blocking for the UI; avoid long blocking work on the main TUI loop.
Stream or page data when feasible.
Use zero-copy and buffering where it improves clarity and speed.

Use Cross-platform dependencies or implement support for multiple platforms.
Aim to make it work on Linux/macOS/Windows.
Avoid platform-specific assumptions; gate them behind `cfg` when necessary.
Be intentional when adding crates.
Prefer standard library and small, well-maintained libraries.

___

By contributing, you agree that your contributions will be licensed under the MIT License, the same as this repository.
