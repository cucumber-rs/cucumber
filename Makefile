###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)




###########
# Aliases #
###########

book: book.build


docs: cargo.doc


fmt: cargo.fmt


lint: cargo.lint


record: record.gif


test: test.cargo




##################
# Cargo commands #
##################

# Generate crates documentation from Rust sources.
#
# Usage:
#	make cargo.doc [crate=<crate-name>]
#	               [private=(yes|no)] [docsrs=(no|yes)]
#	               [open=(yes|no)] [clean=(no|yes)]

cargo.doc:
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	$(if $(call eq,$(docsrs),yes),RUSTDOCFLAGS='--cfg docsrs',) \
	cargo $(if $(call eq,$(docsrs),yes),+nightly,) doc \
		$(if $(call eq,$(crate),),--workspace,-p $(crate)) \
		--all-features \
		$(if $(call eq,$(private),no),,--document-private-items) \
		$(if $(call eq,$(open),no),,--open)


# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Lint Rust sources with Clippy.
#
# Usage:
#	make cargo.lint

cargo.lint:
	cargo clippy --workspace --all-features -- -D warnings




####################
# Testing commands #
####################

# Run Rust tests of project crates.
#
# Usage:
#	make test.cargo [crate=<crate-name>] [careful=(no|yes)]

test.cargo:
ifeq ($(careful),yes)
ifeq ($(shell cargo install --list | grep cargo-careful),)
	cargo install cargo-careful
endif
ifeq ($(shell rustup component list --toolchain=nightly \
              | grep 'rust-src (installed)'),)
	rustup component add --toolchain=nightly rust-src
endif
endif
	cargo $(if $(call eq,$(careful),yes),+nightly careful,) test \
		$(if $(call eq,$(crate),),--workspace,-p $(crate)) --all-features


# Run Rust tests of Book.
#
# Usage:
#	make test.book [clean=(no|yes)]

test.book:
ifeq ($(clean),yes)
	cargo clean
endif
	$(eval target := $(strip $(shell cargo -vV | sed -n 's/host: //p')))
	cargo build --all-features --tests
	OUT_DIR=target mdbook test book -L target/debug/deps $(strip \
		$(if $(call eq,$(findstring windows,$(target)),),,\
			$(shell cargo metadata -q \
			        | jq -r '.packages[] | select(.name == "windows_$(word 1,$(subst -, ,$(target)))_$(word 4,$(subst -, ,$(target)))") | .manifest_path' \
			        | sed -e "s/^/-L '/" -e 's/Cargo.toml/lib/' -e "s/$$/'/" )))




#################
# Book commands #
#################

# Build Book.
#
# Usage:
#	make book.build [out=<dir>]

book.build:
	mdbook build book/ $(if $(call eq,$(out),),,-d $(out))


# Serve Book on some port.
#
# Usage:
#	make book.serve [port=(3000|<port>)]

book.serve:
	mdbook serve book/ -p=$(or $(port),3000)


book.test: test.book


book.tests: test.book




######################
# Recording commands #
######################

# Record GIF image of terminal with asciinema.
#
# Requires asciinema and Docker being installed:
#	https://asciinema.org/docs/installation
#	https://docs.docker.com/get-docker
#
# Usage:
#	make record.gif [name=(<current-datetime>|<file-name>)]

record-gif-dir := book/src/rec
record-gif-name := $(or $(name),$(shell date +%y"-"%m"-"%d"_"%H"-"%M"-"%S))
record-gif-file = $(record-gif-dir)/$(record-gif-name).gif

record.gif:
	asciinema rec --overwrite rec.cast.json
	@mkdir -p $(record-gif-dir)/
	@rm -f $(record-gif-file)
	docker run --rm -v "$(PWD)":/data -w /data \
		asciinema/asciicast2gif -s 2 rec.cast.json $(record-gif-file)
	git add $(record-gif-file)
	@rm -f rec.cast.json
ifeq ($(record-gif-name),readme)
	head -n $$(($$(wc -l < README.md)-1)) README.md > README.tmp.md
	mv README.tmp.md README.md
	printf "[asciicast]: data:image/gif;base64," >> README.md
	base64 -i $(record-gif-file) >> README.md
endif




##################
# .PHONY section #
##################

.PHONY: book docs fmt lint record test \
        cargo.doc cargo.fmt cargo.lint \
        book.build book.serve book.test book.tests \
        record.gif \
        test.cargo test.book
