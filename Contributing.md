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
Since we use the _bors_ bot to merge PRs, we can unfortunately not squash commits upon merge.


### Draft PRs

In case you plan to work for a longer time on a feature/bugfix, consider opening a PR as a draft.
This signals that reviews are appreciated, but that the code is not yet ready for merge.
Non-draft PRs that pass CI are assumed to be mergeable (and maintainers may do so).  
<br/>

# Dev tools

## Local development

The script `check.sh` in the project root can be used to mimic a minimal version of CI locally.
It's useful to run this before you commit, push or create a pull request:

```bash
$ ./check.sh
```

At the time of writing, this will run formatting, clippy, unit tests and integration tests. More checks may be added in the future.
Run `./check.sh --help` to see all available options.

If you like, you can set this as a pre-commit hook in your local clone of the repository:

```bash
$ ln -sf check.sh .git/hooks/pre-commit
```


### Unit tests

Because most of gdext interacts with the Godot engine, which is not available from the test executable, unit tests
(using `cargo test` and the `#[test]` attribute) are pretty limited in scope. They are primarily used for Rust-only logic.

As additional flags might be needed, the preferred way to run unit tests is through the `check.sh` script:

```bash
$ ./check.sh test
```


### Integration tests

The `itest` directory contains a suite of integration tests. It is split into two directories:
`rust`, containing the Rust code for the GDExtension library, and `godot` with the Godot project and GDScript tests.

Similar to `#[test]`, the function annotated by `#[itest]` contains one integration test. There are multiple syntax variations:

```rust
// Use a Godot API and verify the results using assertions.
#[itest]
fn variant_nil() {
    let variant = Variant::nil();
    assert!(variant.is_nil());
}

// TestContext parameter gives access to a node in the scene tree.
#[itest]
fn do_sth_with_the_tree(ctx: &TestContext) {
    let tree: Gd<Node> = ctx.scene_tree.share();
    
    // If you don't need the scene, you can also construct free-standing nodes:
    let node: Gd<Node3D> = Node3D::new_alloc();
    // ...
    node.free(); // don't forget to free everything created by new_alloc().    
}

// Skip a test that's not yet ready.
#[itest(skip)]
fn not_executed() {
    // ...
}

// Focus on a one or a few tests.
// As soon as there is at least one #[itest(focus)], only focused tests are run.
#[itest(focus)]
fn i_need_to_debug_this() {
    // ...
}
```

You can run the integration tests like this:

```bash
$ ./check.sh itest
```

Just like when compiling the crate, the `GODOT4_BIN` environment variable can be used to supply the path and filename of your Godot executable.
Otherwise, a binary named `godot4` in your PATH is used.


### Formatting

`rustfmt` is used to format code. `check.sh` only warns about formatting issues, but does not fix them. To do that, run:

```bash
$ cargo fmt
```


### Clippy

`clippy` is used for additional lint warnings not implemented in `rustc`. This, too, is best run through `check.sh`:

```bash
$ ./check.sh clippy
```

## Continuous Integration

If you want to have the full CI experience, you can experiment as much as you like on your own gdext fork, before submitting a pull request.

For this, navigate to the file `.github/workflows/full-ci.yml` and change the following lines:

```yml
on:
  push:
    branches:
      - staging
      - trying
```

to:

```yml
on:
  push:
```

This runs the entire CI pipeline to run on every push. You can then see the results in the _Actions_ tab in your repository.

Don't forget to undo this before opening a PR! You may want to keep it in a separate commit named "UNDO" or similar.


## Build configurations

### `real` type

Certain types in Godot use either a single or double-precision float internally, such as `Vector2`.
When working with these types, we use the `real` type instead of choosing either `f32` or `f64`.
As a result, our code is portable between Godot binaries compiled with `precision=single` and `precision=double`.

To run the testing suite with `double-precision` enabled you may add `--double` to a `check.sh` invocation:
```bash
$ ./check.sh --double
```

[GitHub issue]: https://github.com/godot-rust/gdext/issues
[Discord]: https://discord.gg/aKUCJ8rJsc
