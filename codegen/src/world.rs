// Copyright (c) 2020-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `#[derive(World)]` macro implementation.

use inflections::case::to_pascal_case;
use itertools::Itertools as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;
use synthez::{ParseAttrs, ToTokens};

/// Generates code of `#[derive(World)]` macro expansion.
///
/// # Errors
///
/// If failed to parse [`Attrs`].
pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(World)]` macro.
#[derive(Debug, Default, ParseAttrs)]
struct Attrs {
    /// Function to be used for a `World` construction.
    ///
    /// If [`None`] then [`Default::default()`] will be used.
    #[parse(value)]
    init: Option<syn::ExprPath>,
}

/// Representation of a type implementing a `World` trait, used for code
/// generation.
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_world_inventory, impl_world, impl_step_constructors))]
struct Definition {
    /// Name of this type.
    ident: syn::Ident,

    /// [`syn::Generics`] of this type.
    generics: syn::Generics,

    /// [`Visibility`] of this `World`.
    ///
    /// [`Visibility`]: syn::Visibility
    vis: syn::Visibility,

    /// Function, which is used to construct `World`. Uses [`Default`] impl, in
    /// case no value is provided.
    init: Option<syn::ExprPath>,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs: Attrs = Attrs::parse_attrs("world", &input)?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            vis: input.vis,
            init: attrs.init,
        })
    }
}

impl Definition {
    /// Possible step names.
    const STEPS: &'static [&'static str] = &["given", "when", "then"];

    /// Assertion to ensure, that [`Self::STEPS`] has exactly 3 step types.
    #[allow(clippy::manual_assert)] // `assert_eq!` isn't const yet
    const EXACTLY_3_STEPS: () = if Self::STEPS.len() != 3 {
        panic!("expected exactly 3 step names");
    };

    /// Generates code of implementing a `WorldInventory` trait.
    fn impl_world_inventory(&self) -> TokenStream {
        let world = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let (given_ty, when_step_ty, then_ty) = self
            .step_types()
            .collect_tuple()
            .unwrap_or_else(|| unreachable!("{:?}", Self::EXACTLY_3_STEPS));

        quote! {
            #[automatically_derived]
            impl #impl_gens ::cucumber::codegen::WorldInventory
                 for #world #ty_gens
                 #where_clause
            {
                type Given = #given_ty;
                type When = #when_step_ty;
                type Then = #then_ty;
            }
        }
    }

    /// Generates code of implementing a `World` trait.
    fn impl_world(&self) -> TokenStream {
        let world = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let init = self.init.clone().unwrap_or_else(
            || parse_quote! { <Self as ::std::default::Default>::default },
        );

        quote! {
            #[automatically_derived]
            #[::cucumber::codegen::async_trait(?Send)]
            impl #impl_gens ::cucumber::World for #world #ty_gens
                 #where_clause
            {
                type Error = ::cucumber::codegen::anyhow::Error;

                async fn new() -> ::std::result::Result<Self, Self::Error> {
                    use ::cucumber::codegen::{
                        IntoWorldResult as _, ToWorldFuture as _,
                    };

                    fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
                        v
                    }

                    (&as_fn_ptr(#init))
                        .to_world_future()
                        .await
                        .into_world_result()
                        .map_err(::std::convert::Into::into)
                }
            }
        }
    }

    /// Generates code for additional struct implementing `StepConstructor`
    /// trait.
    #[must_use]
    fn impl_step_constructors(&self) -> TokenStream {
        let world = &self.ident;
        let world_vis = &self.vis;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        self.step_types()
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
                    impl #impl_gens
                         ::cucumber::codegen::StepConstructor<#world #ty_gens>
                         for #ty #where_clause
                    {
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

    /// Generates [`syn::Ident`]s of generic types for private trait impl.
    ///
    /// [`syn::Ident`]: struct@syn::Ident
    fn step_types(&self) -> impl Iterator<Item = syn::Ident> + '_ {
        Self::STEPS.iter().map(|step| {
            format_ident!("Cucumber{}{}", to_pascal_case(step), self.ident)
        })
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn derives_impl() {
        let input = parse_quote! {
            pub struct World;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cucumber::codegen::WorldInventory for World {
                type Given = CucumberGivenWorld;
                type When = CucumberWhenWorld;
                type Then = CucumberThenWorld;
            }

            #[automatically_derived]
            #[::cucumber::codegen::async_trait(?Send)]
            impl ::cucumber::World for World {
                type Error = ::cucumber::codegen::anyhow::Error;

                async fn new() -> ::std::result::Result<Self, Self::Error> {
                    use ::cucumber::codegen::{
                        IntoWorldResult as _, ToWorldFuture as _,
                    };

                    fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
                        v
                    }

                    (&as_fn_ptr(<Self as ::std::default::Default>::default))
                        .to_world_future()
                        .await
                        .into_world_result()
                        .map_err(::std::convert::Into::into)
                }
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
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_generics() {
        let input = parse_quote! {
            pub struct World<T>(T);
        };

        let output = quote! {
            #[automatically_derived]
            impl<T> ::cucumber::codegen::WorldInventory for World<T> {
                type Given = CucumberGivenWorld;
                type When = CucumberWhenWorld;
                type Then = CucumberThenWorld;
            }

            #[automatically_derived]
            #[::cucumber::codegen::async_trait(?Send)]
            impl<T> ::cucumber::World for World<T> {
                type Error = ::cucumber::codegen::anyhow::Error;

                async fn new() -> ::std::result::Result<Self, Self::Error> {
                    use ::cucumber::codegen::{
                        IntoWorldResult as _, ToWorldFuture as _,
                    };

                    fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
                        v
                    }

                    (&as_fn_ptr(<Self as ::std::default::Default>::default))
                        .to_world_future()
                        .await
                        .into_world_result()
                        .map_err(::std::convert::Into::into)
                }
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_init_fn() {
        let input = parse_quote! {
            #[world(init = Self::custom)]
            pub struct World<T>(T);
        };

        let output = quote! {
            #[automatically_derived]
            impl<T> ::cucumber::codegen::WorldInventory for World<T> {
                type Given = CucumberGivenWorld;
                type When = CucumberWhenWorld;
                type Then = CucumberThenWorld;
            }

            #[automatically_derived]
            #[::cucumber::codegen::async_trait(?Send)]
            impl<T> ::cucumber::World for World<T> {
                type Error = ::cucumber::codegen::anyhow::Error;

                async fn new() -> ::std::result::Result<Self, Self::Error> {
                    use ::cucumber::codegen::{
                        IntoWorldResult as _, ToWorldFuture as _,
                    };

                    fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
                        v
                    }

                    (&as_fn_ptr(Self::custom))
                        .to_world_future()
                        .await
                        .into_world_result()
                        .map_err(::std::convert::Into::into)
                }
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            impl<T> ::cucumber::codegen::StepConstructor<World<T> > for
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
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }
}
