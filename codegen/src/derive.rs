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

use inflections::case::{to_pascal_case, to_snake_case};
use proc_macro2::{Span, TokenStream};
use quote::quote;

/// Generates code of `#[derive(WorldInit)]` macro expansion.
#[allow(clippy::similar_names)]
pub(crate) fn world_init(
    input: TokenStream,
    steps: &[&str],
) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;

    let world = &input.ident;
    let (impl_gen, ty_gen, where_clause) = input.generics.split_for_impl();

    let step_types = step_types(steps, world);
    let step_structs = generate_step_structs(steps, &input);

    let (given_mod, given_ty) = step_types[0].clone();
    let (when_mod, when_ty) = step_types[1].clone();
    let (then_mod, then_ty) = step_types[2].clone();

    Ok(quote! {
        impl<#impl_gen> ::cucumber::codegen::WorldInventory for #world #ty_gen
            #where_clause
        {
            type Given = #given_mod::#given_ty;
            type When = #when_mod::#when_ty;
            type Then = #then_mod::#then_ty;
        }

        #( #step_structs )*
    })
}

/// Generates [`syn::Ident`]s of generic types for private trait impl.
///
/// [`syn::Ident`]: struct@syn::Ident
fn step_types(
    steps: &[&str],
    world: &syn::Ident,
) -> Vec<(syn::Ident, syn::Ident)> {
    steps
        .iter()
        .map(|step| {
            let step = to_pascal_case(step);
            let ty = format!("Cucumber{}{}", step, world);
            let m = to_snake_case(&ty);

            (
                syn::Ident::new(&m, Span::call_site()),
                syn::Ident::new(&ty, Span::call_site()),
            )
        })
        .collect()
}

/// Generates structs and their implementations of private traits.
fn generate_step_structs(
    steps: &[&str],
    world: &syn::DeriveInput,
) -> Vec<TokenStream> {
    let (impl_gen, ty_gen, where_clause) = world.generics.split_for_impl();
    let world = &world.ident;

    step_types(steps, world)
        .iter()
        .map(|(m, ty)| {
            quote! {
                #[automatically_derived]
                #[doc(hidden)]
                pub mod #m {
                    use super::*;

                    #[automatically_derived]
                    #[doc(hidden)]
                    pub struct #ty {
                        #[doc(hidden)]
                        loc: ::cucumber::step::Location,

                        #[doc(hidden)]
                        regex: &'static str,

                        #[doc(hidden)]
                        func: ::cucumber::codegen::SyncHack,
                    }

                    #[automatically_derived]
                    impl #ty {
                       #[doc(hidden)]
                       /// # Safety
                       ///
                       /// `func` argument has to be [`transmute`]d from
                       /// [`cucumber::Step`].
                       ///
                       /// [`transmute`]: std::mem::transmute
                       pub const unsafe fn new (
                           loc: ::cucumber::step::Location,
                           regex: &'static str,
                           func: ::cucumber::codegen::SyncHack,
                       ) -> Self {
                           Self { loc, regex, func }
                       }
                    }

                    #[automatically_derived]
                    impl<#impl_gen> ::cucumber::codegen::StepConstructor<
                        #world #ty_gen
                    > for #ty #where_clause {
                        fn inner(&self) -> (
                            ::cucumber::step::Location,
                            &'static str,
                            ::cucumber::Step<#world #ty_gen>,
                        ) {
                            (
                                self.loc.clone(),
                                self.regex.clone(),
                                // SAFETY
                                // As the only way to construct `Self` in
                                // calling `Self::new()` method, which enforces
                                // right invariants.
                                unsafe { ::std::mem::transmute(self.func) },
                            )
                        }
                    }

                    #[automatically_derived]
                    ::cucumber::codegen::collect!(#ty);
                }
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
            impl<> ::cucumber::codegen::WorldInventory for World {
                type Given = cucumber_given_world::CucumberGivenWorld;
                type When = cucumber_when_world::CucumberWhenWorld;
                type Then = cucumber_then_world::CucumberThenWorld;
            }

            #[automatically_derived]
            #[doc(hidden)]
            pub mod cucumber_given_world {
                use super::*;

                #[automatically_derived]
                #[doc(hidden)]
                pub struct CucumberGivenWorld {
                     #[doc(hidden)]
                     loc: ::cucumber::step::Location,

                     #[doc(hidden)]
                     regex: &'static str,

                     #[doc(hidden)]
                     func: ::cucumber::codegen::SyncHack,
                }

                #[automatically_derived]
                impl CucumberGivenWorld {
                   // TODO: Remove this method, once
                   //       `<Struct as Trait>::Assoc { .. }` is supported.
                   // https://github.com/rust-lang/rust/issues/86935
                   #[doc(hidden)]
                   /// # Safety
                   ///
                   /// `func` argument has to be [`transmute`]d from
                   /// [`cucumber::Step`].
                   ///
                   /// [`transmute`]: std::mem::transmute
                   pub const unsafe fn new (
                       loc: ::cucumber::step::Location,
                       regex: &'static str,
                       func: ::cucumber::codegen::SyncHack,
                   ) -> Self {
                       Self { loc, regex, func }
                   }
                }

                #[automatically_derived]
                impl<> ::cucumber::codegen::StepConstructor<World> for
                    CucumberGivenWorld
                {
                    fn inner(&self) -> (
                        ::cucumber::step::Location,
                        &'static str,
                        ::cucumber::Step<World>,
                    ) {
                        (
                            self.loc.clone(),
                            self.regex.clone(),
                            // SAFETY
                            // As the only way to construct `Self` in
                            // calling `Self::new()` method, which enforces
                            // right invariants.
                            unsafe { ::std::mem::transmute(self.func) },
                        )
                    }
                }

                #[automatically_derived]
                ::cucumber::codegen::collect!(CucumberGivenWorld);
            }

            #[automatically_derived]
            #[doc(hidden)]
            pub mod cucumber_when_world {
                use super::*;

                #[automatically_derived]
                #[doc(hidden)]
                pub struct CucumberWhenWorld {
                     #[doc(hidden)]
                     loc: ::cucumber::step::Location,

                     #[doc(hidden)]
                     regex: &'static str,

                     #[doc(hidden)]
                     func: ::cucumber::codegen::SyncHack,
                }

                #[automatically_derived]
                impl CucumberWhenWorld {
                   #[doc(hidden)]
                   /// # Safety
                   ///
                   /// `func` argument has to be [`transmute`]d from
                   /// [`cucumber::Step`].
                   ///
                   /// [`transmute`]: std::mem::transmute
                   pub const unsafe fn new (
                       loc: ::cucumber::step::Location,
                       regex: &'static str,
                       func: ::cucumber::codegen::SyncHack,
                   ) -> Self {
                       Self { loc, regex, func }
                   }
                }

                #[automatically_derived]
                impl<> ::cucumber::codegen::StepConstructor<World> for
                    CucumberWhenWorld
                {
                    fn inner(&self) -> (
                        ::cucumber::step::Location,
                        &'static str,
                        ::cucumber::Step<World>,
                    ) {
                        (
                            self.loc.clone(),
                            self.regex.clone(),
                            // SAFETY
                            // As the only way to construct `Self` in
                            // calling `Self::new()` method, which enforces
                            // right invariants.
                            unsafe { ::std::mem::transmute(self.func) },
                        )
                    }
                }

                #[automatically_derived]
                ::cucumber::codegen::collect!(CucumberWhenWorld);
            }

            #[automatically_derived]
            #[doc(hidden)]
            pub mod cucumber_then_world {
                use super::*;

                #[automatically_derived]
                #[doc(hidden)]
                pub struct CucumberThenWorld {
                     #[doc(hidden)]
                     loc: ::cucumber::step::Location,

                     #[doc(hidden)]
                     regex: &'static str,

                     #[doc(hidden)]
                     func: ::cucumber::codegen::SyncHack,
                }

                #[automatically_derived]
                impl CucumberThenWorld {
                   #[doc(hidden)]
                   /// # Safety
                   ///
                   /// `func` argument has to be [`transmute`]d from
                   /// [`cucumber::Step`].
                   ///
                   /// [`transmute`]: std::mem::transmute
                   pub const unsafe fn new (
                       loc: ::cucumber::step::Location,
                       regex: &'static str,
                       func: ::cucumber::codegen::SyncHack,
                   ) -> Self {
                       Self { loc, regex, func }
                   }
                }

                #[automatically_derived]
                impl<> ::cucumber::codegen::StepConstructor<World> for
                    CucumberThenWorld
                {
                    fn inner(&self) -> (
                        ::cucumber::step::Location,
                        &'static str,
                        ::cucumber::Step<World>,
                    ) {
                        (
                            self.loc.clone(),
                            self.regex.clone(),
                            // SAFETY
                            // As the only way to construct `Self` in
                            // calling `Self::new()` method, which enforces
                            // right invariants.
                            unsafe { ::std::mem::transmute(self.func) },
                        )
                    }
                }

                #[automatically_derived]
                ::cucumber::codegen::collect!(CucumberThenWorld);
            }
        };

        assert_eq!(
            super::world_init(input, &["given", "when", "then"])
                .unwrap()
                .to_string(),
            output.to_string(),
        );
    }
}
