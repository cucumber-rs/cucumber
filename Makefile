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


test: test.cargo test.book




##################
# Cargo commands #
##################

# Generate crates documentation from Rust sources.
#
# Usage:
#	make cargo.doc [crate=<crate-name>] [private=(yes|no)]
#	               [open=(yes|no)] [clean=(no|yes)]

cargo.doc:
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	cargo doc $(if $(call eq,$(crate),),--workspace,-p $(crate)) \
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
#	make test.cargo [crate=<crate-name>]

test.cargo:
	cargo test $(if $(call eq,$(crate),),--workspace,-p $(crate)) --all-features


# Run Rust tests of Book.
#
# Usage:
#	make test.book

test.book:
	cargo test --manifest-path book/tests/Cargo.toml




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
#	make record [name=(<current-datetime>|<file-name>)]

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
        book.build book.serve \
        record.gif \
        test.cargo test.book
