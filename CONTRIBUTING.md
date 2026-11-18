# Contributing to Rusteal

Thank you for helping build Rusteal. This guide covers:

- how to report a problem;
- how to prepare a pull request;
- how we treat code written with generative AI.

[The Rusteal developer's handbook](https://viniciusmorgado.github.io/dev.rusteal.github.io/) explains how the project works inside, and this guide points to it where it goes deeper.

## Ways to contribute

- **Report a problem or ask for a feature.** Open an issue in this repository.
- **Fix a bug or add a feature.** Open a pull request in this repository: the crates, the `rusteal` CLI, the Unreal Engine plugins and the templates.
- **Improve the documentation.** The handbooks live in their own repositories:
  - [the game developer's handbook](https://github.com/viniciusmorgado/game.rusteal.github.io);
  - [the developer's handbook](https://github.com/viniciusmorgado/dev.rusteal.github.io).

  Pull requests there follow this guide too.

For a large change, such as a new subsystem, a change to the public API or a new platform, open an issue first. Agreeing on the approach before the code is written saves everyone's time.

## Reporting a problem

A good report lets us reproduce the problem. Include:

- the Rusteal version (`rusteal --version`), the Unreal Engine version and the platform;
- what you did, what you expected and what happened;
- the smallest project or snippet that shows it, if you can share one;
- the relevant log lines, `LogRusteal` above all.

## Pull requests

### Setting up

[Setting up a checkout](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/environment.html) lists the prerequisites and the one-time setup. Branch from `develop` and open the pull request against `develop`. `main` only receives releases.

### Commits

Commit subjects follow [Conventional Commits](https://www.conventionalcommits.org/). The release pipeline computes the next version from them:

| Subject | Bump |
|---|---|
| `feat: ...` or `feat(scope): ...` | minor |
| `fix:`, `docs:`, `refactor:`, `test:`, `perf:`, `build:`, `style:`, `chore:` | patch |
| `BREAKING CHANGE: ...` | major |

Pick the type by what the commit does. A pull request that brings a feature contains at least one `feat` commit.

Write code, comments and commit messages in English.

Never write the CI skip marker in a commit message, a pull request's title or its body. GitHub reads it anywhere and runs no checks.

### Before opening it

Run the checks the CI runs, and fix every warning:

```bash
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cargo test -p rusteal-runtime --features rayon,engine-allocator
cargo clippy -p rusteal-runtime --features rayon,engine-allocator --all-targets --no-deps -- -D warnings
```

A change that reaches the engine needs more than the workspace checks. That means a change to the runtime, the macros, the code generator, the plugins or the templates. Test it in a project and, where it applies, with the engine harness. [Testing a change](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/testing.html) explains both, along with its [checklist for a pull request](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/testing.html#before-a-pull-request).

A change that game developers see also updates the game developer's handbook.

### `unsafe` code

`unsafe` code has its own isolated place in Rusteal: the layers that talk to the engine, such as the FFI, the core and the code the generator and the macros emit. It never reaches the game developer:

- a game-facing API needs no `unsafe` and no raw handle from its caller;
- it takes and returns typed references and Rust types;
- its error is a `RustealResult`.

The templates contain no `unsafe` at all, and the `game_developers_never_write_unsafe_code` test checks it. Every `unsafe` block inside Rusteal carries a `// SAFETY:` comment that states the invariants it relies on, whether it was written by hand or generated.

### Assets

Rusteal makes no templates of its own. Its templates are Unreal Engine's templates rewritten in Rust, and their art comes from the Unreal Engine installed on the developer's machine. Rusteal ships code and text files only.

Never commit an art or media asset, whatever its origin and whether or not AI made it. That covers meshes, textures, materials, sounds, music, animations, particle systems and images. There are three reasons:

- its license is hard to verify;
- binary files weigh a code repository down;
- the engine already provides the templates' art.

There are two exceptions:

- **Files a port must change to reach the Rust classes.** These are the template's Blueprints, reparented to the Rust classes, and its levels and their external actors, resaved with them. The few other assets the port resaves, such as state trees and montages, count too. They are rebuilt from the engine's own files by the [porting steps](https://viniciusmorgado.github.io/dev.rusteal.github.io/templates/porting.html), never added as new content.
- **The project's own images.** The images in `assets/` and the launcher's images belong to Rusteal.

### The description

Tell us:

- what the problem is;
- how the change fixes it;
- how you tested it.

Evidence from before and after the change speeds up the merge: a reproduction, a log, a screenshot, a failing test that now passes, or harness output. Add your AI disclosure as well (see below).

## AI policy

Rusteal accepts contributions written by hand and contributions written with generative AI tools. What we care about is the change itself:

- whether it is correct;
- whether it is tested;
- whether its author stands behind it.

The policy has two parts:

- one for **community contributors**;
- one for the **core team**, meaning any developer with a lasting commitment to maintaining the project.

### For community contributors

**You own your pull request.** The core team reviews and tests every pull request before it is merged, whatever its origin. Whether you wrote the code by hand or with a model, you are responsible for its quality, and you are expected to:

- understand every line you submit;
- explain your choices when asked;
- act on the review.

**Tell us whether and how you used AI.** Say so in the pull request description, and say how far it went. For example:

- `AI: none`
- `AI: coding assistant (completions, small suggestions)`
- `AI: most of the code was generated, then reviewed and tested by me`
- `AI: documentation only`
- `AI: pull request description and commit messages only, no code`

This is not held against you. If you are new to Rust, C++ or Unreal Engine, or new to programming, it helps us pace the review and guide you through improvements before the merge. A pull request with no disclosure is read as the work of someone fluent in the languages and technologies the project uses, and the review is written for that reader.

**Answer the review yourself.** Replies to the reviewers come from you, in your own understanding of the change. Pasting a model's output in reply, without understanding it, counts as ignoring the feedback.

**Prefer small, focused pull requests.** Smaller pull requests, sent a few at a time, are easier to review and get merged faster. The best contributions come from real use:

- a fix for a problem you hit in your own game;
- a feature Rusteal does not cover yet and your game needs.

These are worth more than changes made only for the sake of contributing.

**Volume and conduct.** Review time is limited. We may close and archive the pull requests of a user who does any of the following:

- floods the project with more pull requests than can be reviewed;
- repeatedly declines to fix their pull requests;
- ignores feedback.

If the behavior continues, the user may be permanently banned from the project.

### For the core team

**Take extra care on hot paths.** Be careful with changes, and with generated code in particular, in the parts where performance is critical:

- reflection;
- the FFI boundary between Rust and C++;
- the dispatch tables;
- the code the bindings generator emits.

A change there should come with measurements, such as the call costs the engine harness reports, not only with a passing build.

**Automation assists; people decide.** Pull requests may be triaged by automated systems with different priorities, such as security and performance. They may also be reviewed by automated systems, including AI models. The final review, the final test and the decision to merge always belong to a developer of the core team.

**Prove that it works.** Test every change locally and show that it works. Reading the code is not enough, whatever the size of the change. The exceptions are clearly trivial changes, such as comments or a simple rename. When in doubt, open an issue for discussion or ask other developers to look.

### Security reports and fixes

This part applies only to security problems: vulnerabilities in Rusteal and the fixes for them. AI tools may help find them. A security report or a security fix found or written with AI must be verified by the person who sends it:

- a reproduction or a proof of concept against a real build of Rusteal;
- a description of the impact, as observed, not as predicted.

A report that only relays a model's claim, without that verification, is closed.

### Reviewing this policy

AI tools change fast. The core team reviews this policy at each release and updates it when the tools or the project's experience call for it.

## License

Rusteal is licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in Rusteal, as defined in the Apache-2.0 license, is dual licensed as above, without any additional terms or conditions.

Submit only what you have the right to submit under these terms. Never copy Unreal Engine source code into Rusteal, whether from the engine or from Epic's samples. It is licensed under Epic's EULA and cannot enter an MIT or Apache repository. This holds for code a model produced as well: models can reproduce the engine's code almost word for word, and you answer for what you submit.
