// Copyright (c) 2020-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `#[derive(Parameter)]` macro implementation.

use inflections::case::to_lower_case;
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use synthez::{ParseAttrs, Required, ToTokens};

/// Expands `#[derive(Parameter)]` macro.
///
/// # Errors
///
/// If failed to parse [`Attrs`] or the user-provided [`Regex`] is invalid.
pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(Parameter)]` macro.
#[derive(Debug, Default, ParseAttrs)]
struct Attrs {
    /// Value for a `Parameter::REGEX` associated constant.
    #[parse(value)]
    regex: Required<syn::LitStr>,

    /// Value for a `Parameter::NAME` associated constant.
    #[parse(value)]
    name: Option<syn::LitStr>,
}

/// Representation of a type implementing a `Parameter` trait, used for code
/// generation.
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_parameter))]
struct Definition {
    /// Name of this type.
    ident: syn::Ident,

    /// [`syn::Generics`] of this type.
    generics: syn::Generics,

    /// Value for a `Parameter::REGEX` associated constant.
    regex: Regex,

    /// Value for a `Parameter::Name` associated constant.
    name: String,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs: Attrs = Attrs::parse_attrs("param", &input)?;

        let regex = Regex::new(&attrs.regex.value()).map_err(|e| {
            syn::Error::new(attrs.regex.span(), format!("invalid regex: {e}"))
        })?;

        let name = attrs.name.as_ref().map_or_else(
            || to_lower_case(&input.ident.to_string()),
            syn::LitStr::value,
        );

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            regex,
            name,
        })
    }
}

impl Definition {
    /// Generates code of implementing a `Parameter` trait.
    #[must_use]
    fn impl_parameter(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();
        let (regex, name) = (self.regex.as_str(), &self.name);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::cucumber::Parameter for #ty #ty_gens
                 #where_clause
            {
                const REGEX: &'static str = #regex;
                const NAME: &'static str = #name;
            }
        }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn derives_impl() {
        let input = parse_quote! {
            #[param(regex = "cat|dog", name = "custom")]
            struct Parameter;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cucumber::Parameter for Parameter {
                const REGEX: &'static str = "cat|dog";
                const NAME: &'static str = "custom";
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_default_name() {
        let input = parse_quote! {
            #[param(regex = "cat|dog")]
            struct Animal;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cucumber::Parameter for Animal {
                const REGEX: &'static str = "cat|dog";
                const NAME: &'static str = "animal";
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_capturing_group() {
        let input = parse_quote! {
            #[param(regex = "(cat)|(dog)")]
            struct Animal;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::cucumber::Parameter for Animal {
                const REGEX: &'static str = "(cat)|(dog)";
                const NAME: &'static str = "animal";
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_generics() {
        let input = parse_quote! {
            #[param(regex = "cat|dog", name = "custom")]
            struct Parameter<T>(T);
        };

        let output = quote! {
            #[automatically_derived]
            impl<T> ::cucumber::Parameter for Parameter<T> {
                const REGEX: &'static str = "cat|dog";
                const NAME: &'static str = "custom";
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_non_capturing_regex_groups() {
        let input = parse_quote! {
            #[param(regex = "cat|dog(?:s)?", name = "custom")]
            struct Parameter<T>(T);
        };

        let output = quote! {
            #[automatically_derived]
            impl<T> ::cucumber::Parameter for Parameter<T> {
                const REGEX: &'static str = "cat|dog(?:s)?";
                const NAME: &'static str = "custom";
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn regex_arg_is_required() {
        let input = parse_quote! {
            #[param(name = "custom")]
            struct Parameter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "`regex` argument of `#[param]` attribute is expected to be \
             present, but is absent",
        );
    }

    #[test]
    fn invalid_regex() {
        let input = parse_quote! {
            #[param(regex = "(cat|dog")]
            struct Parameter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "\
invalid regex: regex parse error:
    (cat|dog
    ^
error: unclosed group",
        );
    }
}
