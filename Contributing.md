# Contributing to godot-rust

We appreciate if users experiment with the library, use it in small projects and report issues they encounter.
If you intend to contribute code, please read the section _Pull request guidelines_ below, so you spend less time on administrative tasks.

The rest of the document goes into tools and infrastructure available for development.


## Pull request guidelines

### 📜 Larger changes need design

If you plan to make bigger contributions, make sure to discuss them in a [GitHub issue] before opening a pull request (PR).
Since the library is evolving quickly, this avoids that multiple people work on the same thing, or that features don't integrate well,
causing a lot of rework. Also don't hesitate to talk to the developers in the `#contrib-gdext` channel on [Discord]!


### 🧮 One commit per logical change

Commits as logical units make it easier to review changes. In the future, they allow to reconstruct what happened when and -- 
in case of regressions -- revert individual changes. The exception are tiny changes of a few lines that don't bear semantic significance
(typos, style, etc.). Larger code style changes should be split though (possibly even in individual pull requests).

Since we use GitHub merge queues, we can unfortunately not use GitHub's "Squash & Merge" feature.

If your pull request changes a single logical change, please squash the commits into one. This is quite simple:
```bash
git reset --soft master      # keeps current changes, but moves HEAD
git commit -am "Descriptive commit message"
git push --force-with-lease  # updates PR branch to this 1 new commit
```

Avoid commits like "integrate review feedback" or "fix rustfmt". If you have multiple commits, use `git commit --amend` or `git rebase -i`
and force-push follow-up commits to your branch (`--force-with-lease` is safer than `-f` or `--force`).


### ✏️ Draft PRs

If you plan to work on a feature/bugfix for a longer time, consider opening a PR as a draft.
This signals that reviews are appreciated, but that the code is not yet ready for merge.
Non-draft PRs that pass CI are assumed to be mergeable (and maintainers may do so).

If you have to abandon a pull request you started, that's totally fine -- but please communicate this,
so that it doesn't stay in limbo. Maybe someone else can step up to finish it :)


### 🤖 AI policy

If you use AI assistants or agents, it is your responsibility to:
- Carefully review and adjust the code **before** submitting a pull request.
- Understand **100%** of the submitted code and be able to explain it in your own words. Do not generate the PR description with AI.
- Uphold any copyrights and licenses. For involved algorithms, do research to properly credit (or directly use) sources.

The section _[Larger changes need design](#-larger-changes-need-design)_ is even more important here. Dropping big amounts of code without a clear approach
can result in your PR being closed with zero reviews.


## Development tools

Further information for contributors, such as tools supporting you in local development, is available in the [godot-rust book].  
The book also elaborates design principles and conventions behind our API.

[GitHub issue]: https://github.com/godot-rust/gdext/issues
[Discord]: https://discord.gg/aKUCJ8rJsc
[godot-rust book]: https://godot-rust.github.io/book/contribute