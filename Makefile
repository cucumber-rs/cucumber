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

docs: cargo.doc


fmt: cargo.fmt


lint: cargo.lint




##################
# Cargo commands #
##################

# Generate crates documentation from Rust sources.
#
# Usage:
#	make cargo.doc [crate=<crate-name>] [open=(yes|no)] [clean=(no|yes)]

cargo.doc:
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	cargo +stable doc $(if $(call eq,$(crate),),--workspace,-p $(crate)) \
		--all-features \
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
	cargo +stable clippy --workspace -- -D clippy::pedantic -D warnings




####################
# Testing commands #
####################

# Run Rust tests of project.
#
# Usage:
#	make test [crate=<crate-name>]

test:
	cargo +stable test $(if $(call eq,$(crate),),--workspace,-p $(crate)) \
		--all-features




##################
# .PHONY section #
##################

.PHONY: docs fmt lint test \
        cargo.doc cargo.fmt cargo.lint
