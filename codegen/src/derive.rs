// Copyright (c) 2020  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
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
pub(crate) fn world_init(
    input: TokenStream,
    steps: &[&str],
) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;

    let world = &input.ident;

    let step_types = step_types(steps, world);
    let step_structs = generate_step_structs(steps, &input);

    Ok(quote! {
        impl ::cucumber::codegen::WorldInventory<
            #( #step_types, )*
        > for #world {}

        #( #step_structs )*
    })
}

/// Generates [`syn::Ident`]s of generic types for private trait impl.
///
/// [`syn::Ident`]: struct@syn::Ident
fn step_types(steps: &[&str], ident: &syn::Ident) -> Vec<syn::Ident> {
    steps
        .iter()
        .map(|step| {
            let step = to_pascal_case(step);
            format_ident!("Cucumber{}{}", step, ident)
        })
        .collect()
}

/// Generates structs and their implementations of private traits.
fn generate_step_structs(
    steps: &[&str],
    world: &syn::DeriveInput,
) -> Vec<TokenStream> {
    let (world, world_vis) = (&world.ident, &world.vis);

    step_types(steps, world)
        .iter()
        .map(|ty| {
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                #world_vis struct #ty {
                    #[doc(hidden)]
                    pub loc: ::cucumber::step::Location,

                    #[doc(hidden)]
                    pub regex: ::cucumber::codegen::Regex,

                    #[doc(hidden)]
                    pub func: ::cucumber::Step<#world>,
                }

                #[automatically_derived]
                impl ::cucumber::codegen::StepConstructor<#world> for #ty {
                    fn new (
                        loc: ::cucumber::step::Location,
                        regex: ::cucumber::codegen::Regex,
                        func: ::cucumber::Step<#world>,
                    ) -> Self {
                        Self { loc, regex, func }
                    }

                    fn inner(&self) -> (
                        ::cucumber::step::Location,
                        ::cucumber::codegen::Regex,
                        ::cucumber::Step<#world>,
                    ) {
                        (
                            self.loc.clone(),
                            self.regex.clone(),
                            self.func.clone(),
                        )
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
    fn expand() {
        let input = parse_quote! {
            pub struct World;
        };

        let output = quote! {
            impl ::cucumber::codegen::WorldInventory<
                CucumberGivenWorld, CucumberWhenWorld, CucumberThenWorld,
            > for World {}

            #[automatically_derived]
            #[doc(hidden)]
            pub struct CucumberGivenWorld {
                #[doc(hidden)]
                pub loc: ::cucumber::step::Location,

                #[doc(hidden)]
                pub regex: ::cucumber::codegen::Regex,

                #[doc(hidden)]
                pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberGivenWorld
            {
                fn new (
                    loc: ::cucumber::step::Location,
                    regex: ::cucumber::codegen::Regex,
                    func: ::cucumber::Step<World>,
                ) -> Self {
                    Self { loc, regex, func }
                }

                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::Regex,
                    ::cucumber::Step<World>,
                ) {
                    (
                        self.loc.clone(),
                        self.regex.clone(),
                        self.func.clone(),
                    )
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
                pub regex: ::cucumber::codegen::Regex,

                #[doc(hidden)]
                pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberWhenWorld
            {
                fn new (
                    loc: ::cucumber::step::Location,
                    regex: ::cucumber::codegen::Regex,
                    func: ::cucumber::Step<World>,
                ) -> Self {
                    Self { loc, regex, func }
                }

                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::Regex,
                    ::cucumber::Step<World>,
                ) {
                    (
                        self.loc.clone(),
                        self.regex.clone(),
                        self.func.clone(),
                    )
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
                pub regex: ::cucumber::codegen::Regex,

                #[doc(hidden)]
                pub func: ::cucumber::Step<World>,
            }

            #[automatically_derived]
            impl ::cucumber::codegen::StepConstructor<World> for
                CucumberThenWorld
            {
                fn new (
                    loc: ::cucumber::step::Location,
                    regex: ::cucumber::codegen::Regex,
                    func: ::cucumber::Step<World>,
                ) -> Self {
                    Self { loc, regex, func }
                }

                fn inner(&self) -> (
                    ::cucumber::step::Location,
                    ::cucumber::codegen::Regex,
                    ::cucumber::Step<World>,
                ) {
                    (
                        self.loc.clone(),
                        self.regex.clone(),
                        self.func.clone(),
                    )
                }
            }

            #[automatically_derived]
            ::cucumber::codegen::collect!(CucumberThenWorld);
        };

        assert_eq!(
            super::world_init(input, &["given", "when", "then"])
                .unwrap()
                .to_string(),
            output.to_string(),
        );
    }
}
