# Contributing to gdext

We appreciate if users experiment with the library, use it in small projects and report issues they encounter.
If you intend to contribute code, please read the section _Pull request guidelines_ below, so you spend less time on administrative tasks.

The rest of the document goes into tools and infrastructure available for development.


## Pull request guidelines

### Larger changes need design

If you plan to make bigger contributions, make sure to discuss them in a [GitHub issue] before opening a pull request (PR).
Since the library is evolving quickly, this avoids that multiple people work on the same thing, or that features don't integrate well,
causing a lot of rework. Also don't hesitate to talk to the developers in the `#contrib-gdext` channel on [Discord]!


### One commit per logical change

This makes it easier to review changes, later reconstruct what happened when and -- in case of regressions -- revert individual commits.
The exception are tiny changes of a few lines that don't bear semantic significance (typos, style, etc.).
Larger code style changes should be split though.

If your pull request changes a single thing, please squash the commits into one. Avoid commits like "integrate review feedback" or "fix rustfmt".
Instead, use `git commit --amend` or `git rebase -i` and force-push follow-up commits to your branch (`git push --force-with-lease`).
Since we use GitHub merge queues, we can unfortunately not decide to squash commits upon merge per PR.


### Draft PRs

In case you plan to work for a longer time on a feature/bugfix, consider opening a PR as a draft.
This signals that reviews are appreciated, but that the code is not yet ready for merge.
Non-draft PRs that pass CI are assumed to be mergeable (and maintainers may do so).  
<br/>


## Development tools

Further information for contributors, such as tools supporting you in local development, is available in the [godot-rust book].
The book also elaborates design principles and conventions behind our API.

[GitHub issue]: https://github.com/godot-rust/gdext/issues
[Discord]: https://discord.gg/aKUCJ8rJsc
[godot-rust book]: https://godot-rust.github.io/book/contribute