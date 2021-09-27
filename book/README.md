# cucumber book

Book containing the [`cucumber_rust`](https://crates.io/crates/cucumber_rust) documentation.




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

To run the tests validating all code examples in the book, run:

```bash
cd tests/
cargo test

# or from project root dir:
make test.book
```




## Test setup

All Rust code examples in the Book are compiled on the CI.

This is done using the [`skeptic`](https://github.com/budziq/rust-skeptic) crate.
