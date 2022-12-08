# `cucumber` Book

Book containing the [`cucumber` crate](https://crates.io/crates/cucumber) user guide.




## Contributing


### Requirements

The Book is built with [mdBook].

You can install it with:
```bash
cargo install mdbook
```


### Local test server

To launch a local test server that continually re-builds the Book and auto-reloads the page, run:
```bash
mdbook serve

# or from project root dir:
make book.serve
```


### Building

You can build the Book to rendered HTML with this command:
```bash
mdbook build

# or from project root dir:
make book
```

The output will be in the `_rendered/` directory.


### Syntax highlighting

As the [default supported languages][1] for [mdBook]'s build of [`highlight.js`] don't include [Gherkin], we build our own version:
```bash
# from project root dir:
make book.highlight.js

# or concrete version:
make book.highlight.js ver=10.7.3
```

> __WARNING__: [mdBook] doesn't work with [`highlight.js`] of versions > `10` (see [rust-lang/mdBook#1622](https://github.com/rust-lang/mdBook/issues/1622) for details).


### Running tests

To run the tests validating all code examples in the book, run (from project root dir):

```bash
cargo build --all-features --tests
OUT_DIR=target mdbook test -L target/debug/deps

# or via shortcut:
make test.book
```




[`highlight.js`]: https://github.com/highlightjs/highlight.js
[Gherkin]: https://cucumber.io/docs/gherkin
[mdBook]: https://github.com/rust-lang/mdBook

[1]: https://rust-lang.github.io/mdBook/format/theme/syntax-highlighting.html#supported-languages
