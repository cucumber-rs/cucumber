// Copyright (c) 2020-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `#[derive(WorldInit)]` macro implementation.

use inflections::case::to_pascal_case;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates code of `#[derive(WorldInit)]` macro expansion.
#[allow(clippy::similar_names)]
pub(crate) fn derive(
    input: TokenStream,
    steps: &[&str],
) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;

    let world = &input.ident;

    let step_types = step_types(steps, world);
    let step_structs = generate_step_structs(steps, &input);

    let given_ty = &step_types[0];
    let when_ty = &step_types[1];
    let then_ty = &step_types[2];

    Ok(quote! {
        impl ::cucumber::codegen::WorldInventory for #world {
            type Given = #given_ty;
            type When = #when_ty;
            type Then = #then_ty;
        }

        #( #step_structs )*
    })
}

/// Generates [`syn::Ident`]s of generic types for private trait impl.
///
/// [`syn::Ident`]: struct@syn::Ident
fn step_types(steps: &[&str], world: &syn::Ident) -> Vec<syn::Ident> {
    steps
        .iter()
        .map(|step| format_ident!("Cucumber{}{}", to_pascal_case(step), world))
        .collect()
}

/// Generates structs and their implementations of private traits.
fn generate_step_structs(
    steps: &[&str],
    world: &syn::DeriveInput,
) -> Vec<TokenStream> {
    let world_vis = &world.vis;
    let world = &world.ident;

    step_types(steps, world)
        .iter()
        .map(|ty| {
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                #world_vis struct #ty {
                    #[doc(hidden)]
                    #world_vis loc: ::cucumber::step::Location,

                    #[doc(hidden)]
                    #world_vis regex: ::cucumber::codegen::LazyRegex,

                    #[doc(hidden)]
                    #world_vis func: ::cucumber::Step<#world>,
                }

                #[automatically_derived]
                impl ::cucumber::codegen::StepConstructor<#world> for #ty {
                    fn inner(&self) -> (
                        ::cucumber::step::Location,
                        ::cucumber::codegen::LazyRegex,
                        ::cucumber::Step<#world>,
                    ) {
                        (self.loc, self.regex, self.func)
                    }
                }

                #[automatically_derived]
                ::cucumber::codegen::collect!(#ty);
            }
        })
        .collect()
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn expands() {
        let input = parse_quote! {
            pub struct World;
        };

        let output = quote! {
            impl ::cucumber::codegen::WorldInventory for World {
                type Given = CucumberGivenWorld;
                type When = CucumberWhenWorld;
                type Then = CucumberThenWorld;
            }

            #[automatically_derived]
            #[doc(hidden)]
            pub struct CucumberGivenWorld {
                 #[doc(hidden)]
                 pub loc: ::cucumber::step::Location,

                 #[doc(hidden)]
                 pub regex: ::cucumber::codegen::LazyRegex,

                 #[doc(hidden)]
                 pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberGivenWorld
            {
                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::LazyRegex,
                    ::cucumber::Step<World>,
                ) {
                    (self.loc, self.regex, self.func)
                }
            }

            #[automatically_derived]
            ::cucumber::codegen::collect!(CucumberGivenWorld);

            #[automatically_derived]
            #[doc(hidden)]
            pub struct CucumberWhenWorld {
                 #[doc(hidden)]
                 pub loc: ::cucumber::step::Location,

                 #[doc(hidden)]
                 pub regex: ::cucumber::codegen::LazyRegex,

                 #[doc(hidden)]
                 pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberWhenWorld
            {
                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::LazyRegex,
                    ::cucumber::Step<World>,
                ) {
                    (self.loc, self.regex, self.func)
                }
            }

            #[automatically_derived]
            ::cucumber::codegen::collect!(CucumberWhenWorld);

            #[automatically_derived]
            #[doc(hidden)]
            pub struct CucumberThenWorld {
                 #[doc(hidden)]
                 pub loc: ::cucumber::step::Location,

                 #[doc(hidden)]
                 pub regex: ::cucumber::codegen::LazyRegex,

                 #[doc(hidden)]
                 pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberThenWorld
            {
                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::LazyRegex,
                    ::cucumber::Step<World>,
                ) {
                    (self.loc, self.regex, self.func)
                }
            }

            #[automatically_derived]
            ::cucumber::codegen::collect!(CucumberThenWorld);
        };

        assert_eq!(
            super::derive(input, &["given", "when", "then"])
                .unwrap()
                .to_string(),
            output.to_string(),
        );
    }
}
