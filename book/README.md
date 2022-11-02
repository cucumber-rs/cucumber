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


### Running tests

To run the tests validating all code examples in the book, run (from project root dir):

```bash
cargo build --all-features --tests
OUT_DIR=target mdbook test -L target/debug/deps

# or via shortcut:
make test.book
```
