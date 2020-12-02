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
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

/// Generates code of `#[derive(WorldInit)]` macro expansion.
pub(crate) fn world_init(input: TokenStream, steps: &[&str]) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;

    let step_types = step_types(steps);
    let step_structs = generate_step_structs(steps, &input);

    let world = &input.ident;

    Ok(quote! {
        impl ::cucumber_rust::private::WorldInventory<
            #( #step_types, )*
        > for #world {}

        #( #step_structs )*
    })
}

/// Generates [`syn::Ident`]s of generic types for private trait impl.
fn step_types(steps: &[&str]) -> Vec<syn::Ident> {
    steps
        .iter()
        .flat_map(|step| {
            let step = to_pascal_case(step);
            vec![
                format_ident!("Cucumber{}", step),
                format_ident!("Cucumber{}Regex", step),
                format_ident!("Cucumber{}Async", step),
                format_ident!("Cucumber{}RegexAsync", step),
            ]
        })
        .collect()
}

/// Generates structs and their implementations of private traits.
fn generate_step_structs(steps: &[&str], world: &syn::DeriveInput) -> Vec<TokenStream> {
    let (world, world_vis) = (&world.ident, &world.vis);

    let idents = [
        (
            syn::Ident::new("Step", Span::call_site()),
            syn::Ident::new("CucumberFn", Span::call_site()),
        ),
        (
            syn::Ident::new("StepRegex", Span::call_site()),
            syn::Ident::new("CucumberRegexFn", Span::call_site()),
        ),
        (
            syn::Ident::new("StepAsync", Span::call_site()),
            syn::Ident::new("CucumberAsyncFn", Span::call_site()),
        ),
        (
            syn::Ident::new("StepRegexAsync", Span::call_site()),
            syn::Ident::new("CucumberAsyncRegexFn", Span::call_site()),
        ),
    ];

    step_types(steps)
        .iter()
        .zip(idents.iter().cycle())
        .map(|(ty, (trait_ty, func))| {
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                #world_vis struct #ty {
                    #[doc(hidden)]
                    pub name: &'static str,

                    #[doc(hidden)]
                    pub func: ::cucumber_rust::private::#func<#world>,
                }

                #[automatically_derived]
                impl ::cucumber_rust::private::#trait_ty<#world> for #ty {
                    fn new (
                        name: &'static str,
                        func: ::cucumber_rust::private::#func<#world>,
                    ) -> Self {
                        Self { name, func }
                    }

                    fn inner(&self) -> (
                        &'static str,
                        ::cucumber_rust::private::#func<#world>,
                    ) {
                        (self.name, self.func.clone())
                    }
                }

                #[automatically_derived]
                ::cucumber_rust::private::collect!(#ty);
            }
        })
        .collect()
}
