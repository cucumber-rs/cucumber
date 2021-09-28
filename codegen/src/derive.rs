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

    let step_types = step_types(steps);
    let step_structs = generate_step_structs(steps, &input);

    let world = &input.ident;

    Ok(quote! {
        impl ::cucumber::codegen::WorldInventory<
            #( #step_types, )*
        > for #world {}

        #( #step_structs )*
    })
}

/// Generates [`syn::Ident`]s of generic types for private trait impl.
fn step_types(steps: &[&str]) -> Vec<syn::Ident> {
    steps
        .iter()
        .map(|step| {
            let step = to_pascal_case(step);
            format_ident!("Cucumber{}", step)
        })
        .collect()
}

/// Generates structs and their implementations of private traits.
fn generate_step_structs(
    steps: &[&str],
    world: &syn::DeriveInput,
) -> Vec<TokenStream> {
    let (world, world_vis) = (&world.ident, &world.vis);

    step_types(steps)
        .iter()
        .map(|ty| {
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                #world_vis struct #ty {
                    #[doc(hidden)]
                    pub regex: ::cucumber::codegen::Regex,

                    #[doc(hidden)]
                    pub func: ::cucumber::Step<#world>,
                }

                #[automatically_derived]
                impl ::cucumber::codegen::StepConstructor<#world> for #ty {
                    fn new (
                        regex: ::cucumber::codegen::Regex,
                        func: ::cucumber::Step<#world>,
                    ) -> Self {
                        Self { regex, func }
                    }

                    fn inner(&self) -> (
                        ::cucumber::codegen::Regex,
                        ::cucumber::Step<#world>,
                    ) {
                        (self.regex.clone(), self.func.clone())
                    }
                }

                #[automatically_derived]
                ::cucumber::codegen::collect!(#ty);
            }
        })
        .collect()
}
