# cucumber book

Book containing the [`cucumber`](https://crates.io/crates/cucumber) documentation.




## Contributing


### Requirements

The Book is built with [mdBook](https://github.com/rust-lang-nursery/mdBook).

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

As [default support languages](https://rust-lang.github.io/mdBook/format/theme/syntax-highlighting.html#supported-languages) for [mdBook](https://github.com/rust-lang-nursery/mdBook)'s build of [`highlight.js`](https://github.com/highlightjs/highlight.js) doesn't include [`gherkin`](https://cucumber.io/docs/gherkin/), we build our own version:

```bash
git clone git@github.com:highlightjs/highlight.js.git
cd highlight.js
git checkout 10.7.3
npm install
node tools/build.js :common gherkin
cp build/highlight.min.js ../book/theme/highlight.js
cd ../ && rm -rf highlight.js/
```

> __NOTE__: [mdBook](https://github.com/rust-lang-nursery/mdBook) doesn't work with versions of [`highlight.js`](https://github.com/highlightjs/highlight.js) from `0.11`: [rust-lang/mdBook#1622](https://github.com/rust-lang/mdBook/issues/1622)


### Running tests

To run the tests validating all code examples in the book, run (from project root dir):

```bash
cargo build --all-features --tests
OUT_DIR=target mdbook test -L target/debug/deps

# or via shortcut:
make test.book
```
