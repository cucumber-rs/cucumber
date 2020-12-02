// Copyright (c) 2020  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `#[given]`, `#[when]` and `#[then]` attribute macros implementation.

use std::mem;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

/// Generates code of `#[given]`, `#[when]` and `#[then]` attribute macros expansion.
pub(crate) fn step(
    attr_name: &'static str,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<TokenStream> {
    Step::parse(attr_name, args, input).and_then(Step::expand)
}

/// Parsed state (ready for code generation) of the attribute and the function it's applied to.
#[derive(Clone, Debug)]
struct Step {
    /// Name of the attribute (`given`, `when` or `then`).
    attr_name: &'static str,

    /// Argument of the attribute.
    attr_arg: AttributeArgument,

    /// Function the attribute is applied to.
    func: syn::ItemFn,

    /// Name of the function argument representing a [`gherkin::Step`][1] reference.
    ///
    /// [1]: cucumber_rust::gherking::Step
    step_arg_name: Option<syn::Ident>,
}

impl Step {
    /// Parses [`Step`] definition from the attribute macro input.
    fn parse(attr_name: &'static str, attr: TokenStream, body: TokenStream) -> syn::Result<Self> {
        let attr_arg = syn::parse2::<AttributeArgument>(attr)?;
        let mut func = syn::parse2::<syn::ItemFn>(body)?;

        let step_arg_name = {
            let (arg_marked_as_step, _) = remove_all_attrs((attr_name, "step"), &mut func);

            match arg_marked_as_step.len() {
                0 => Ok(None),
                1 => {
                    // Unwrapping is OK here, because
                    // `arg_marked_as_step.len() == 1`.
                    let (ident, _) = parse_fn_arg(arg_marked_as_step.first().unwrap())?;
                    Ok(Some(ident.clone()))
                }
                _ => Err(syn::Error::new(
                    // Unwrapping is OK here, because
                    // `arg_marked_as_step.len() > 1`.
                    arg_marked_as_step.get(1).unwrap().span(),
                    "Only 1 step argument is allowed",
                )),
            }
        }?
        .or_else(|| {
            func.sig.inputs.iter().find_map(|arg| {
                if let Ok((ident, _)) = parse_fn_arg(arg) {
                    if ident == "step" {
                        return Some(ident.clone());
                    }
                }
                None
            })
        });

        Ok(Self {
            attr_arg,
            attr_name,
            func,
            step_arg_name,
        })
    }

    /// Expands generated code of this [`Step`] definition.
    fn expand(self) -> syn::Result<TokenStream> {
        let is_regex = matches!(self.attr_arg, AttributeArgument::Regex(_));

        let func = &self.func;
        let func_name = &func.sig.ident;

        let mut func_args = TokenStream::default();
        let (mut addon_args, mut addon_parsing) = (None, None);
        let mut is_step_arg_considered = false;
        if is_regex {
            addon_args = Some(if func.sig.asyncness.is_some() {
                quote! { __cucumber_matches, }
            } else {
                quote! { __cucumber_matches: Vec<String>, }
            });

            if let Some(elem_ty) = parse_slice_from_second_arg(&func.sig) {
                addon_parsing = Some(quote! {
                    let __cucumber_matches = __cucumber_matches
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, s)| {
                            s.parse::<#elem_ty>().unwrap_or_else(|e| panic!(
                                "Failed to parse {} element '{}': {}", i, s, e,
                            ))
                        })
                        .collect::<Vec<_>>();
                });
                func_args = quote! {
                    __cucumber_matches.as_slice(),
                }
            } else {
                #[allow(clippy::redundant_closure_for_method_calls)]
                let (idents, parsings): (Vec<_>, Vec<_>) = itertools::process_results(
                    func.sig
                        .inputs
                        .iter()
                        .skip(1)
                        .map(|arg| self.arg_ident_and_parse_code(arg)),
                    |i| i.unzip(),
                )?;
                is_step_arg_considered = true;

                addon_parsing = Some(quote! {
                    let mut __cucumber_iter = __cucumber_matches.iter().skip(1);
                    #( #parsings )*
                });
                func_args = quote! {
                    #( #idents, )*
                }
            }
        }
        if self.step_arg_name.is_some() && !is_step_arg_considered {
            func_args = quote! {
                #func_args
                ::std::borrow::Borrow::borrow(&__cucumber_step),
            };
        }

        let world = parse_world_from_args(&self.func.sig)?;
        let constructor_method = self.constructor_method();

        let step_matcher = self.attr_arg.literal().value();
        let step_caller = if func.sig.asyncness.is_none() {
            let caller_name = format_ident!("__cucumber_{}_{}", self.attr_name, func_name);
            quote! {
                {
                    #[automatically_derived]
                    fn #caller_name(
                        mut __cucumber_world: #world,
                        #addon_args
                        __cucumber_step: ::std::rc::Rc<::cucumber_rust::gherkin::Step>,
                    ) -> #world {
                        #addon_parsing
                        #func_name(&mut __cucumber_world, #func_args);
                        __cucumber_world
                    }

                    #caller_name
                }
            }
        } else {
            quote! {
                ::cucumber_rust::t!(
                    |mut __cucumber_world, #addon_args __cucumber_step| {
                        #addon_parsing
                        #func_name(&mut __cucumber_world, #func_args).await;
                        __cucumber_world
                    }
                )
            }
        };

        Ok(quote! {
            #func

            #[automatically_derived]
            ::cucumber_rust::private::submit!(
                #![crate = ::cucumber_rust::private] {
                    <#world as ::cucumber_rust::private::WorldInventory<
                        _, _, _, _, _, _, _, _, _, _, _, _,
                    >>::#constructor_method(#step_matcher, #step_caller)
                }
            );
        })
    }

    /// Composes name of the [`WorldInventory`] method to wire this [`Step`]
    /// with.
    fn constructor_method(&self) -> syn::Ident {
        let regex = match &self.attr_arg {
            AttributeArgument::Regex(_) => "_regex",
            AttributeArgument::Literal(_) => "",
        };
        format_ident!(
            "new_{}{}{}",
            self.attr_name,
            regex,
            self.func
                .sig
                .asyncness
                .as_ref()
                .map(|_| "_async")
                .unwrap_or_default(),
        )
    }

    /// Returns [`syn::Ident`] and parsing code of the given function's
    /// argument.
    ///
    /// Function's argument type have to implement [`FromStr`].
    ///
    /// [`FromStr`]: std::str::FromStr
    fn arg_ident_and_parse_code<'a>(
        &self,
        arg: &'a syn::FnArg,
    ) -> syn::Result<(&'a syn::Ident, TokenStream)> {
        let (ident, ty) = parse_fn_arg(arg)?;

        let is_step_arg = self.step_arg_name.as_ref().map(|i| *i == *ident) == Some(true);

        let decl = if is_step_arg {
            quote! {
                let #ident = ::std::borrow::Borrow::borrow(&__cucumber_step);
            }
        } else {
            let ty = match ty {
                syn::Type::Path(p) => p,
                _ => return Err(syn::Error::new(ty.span(), "Type path expected")),
            };

            let not_found_err = format!("{} not found", ident);
            let parsing_err = format!(
                "{} can not be parsed to {}",
                ident,
                ty.path.segments.last().unwrap().ident
            );

            quote! {
                let #ident = __cucumber_iter
                    .next()
                    .expect(#not_found_err)
                    .parse::<#ty>()
                    .expect(#parsing_err);
            }
        };

        Ok((ident, decl))
    }
}

/// Argument of the attribute macro.
#[derive(Clone, Debug)]
enum AttributeArgument {
    /// `#[step("literal")]` case.
    Literal(syn::LitStr),

    /// `#[step(regex = "regex")]` case.
    Regex(syn::LitStr),
}

impl AttributeArgument {
    /// Returns the underlying [`syn::LitStr`].
    fn literal(&self) -> &syn::LitStr {
        match self {
            Self::Regex(l) | Self::Literal(l) => l,
        }
    }
}

impl Parse for AttributeArgument {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let arg = input.parse::<syn::NestedMeta>()?;
        match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(arg)) => {
                if arg.path.is_ident("regex") {
                    let str_lit = to_string_literal(arg.lit)?;

                    let _ = regex::Regex::new(str_lit.value().as_str()).map_err(|e| {
                        syn::Error::new(str_lit.span(), format!("Invalid regex: {}", e.to_string()))
                    })?;

                    Ok(AttributeArgument::Regex(str_lit))
                } else {
                    Err(syn::Error::new(arg.span(), "Expected regex argument"))
                }
            }

            syn::NestedMeta::Lit(l) => Ok(AttributeArgument::Literal(to_string_literal(l)?)),

            syn::NestedMeta::Meta(_) => Err(syn::Error::new(
                arg.span(),
                "Expected string literal or regex argument",
            )),
        }
    }
}

/// Removes all `#[attr_path(attr_arg)]` attributes from the given function
/// signature and returns these attributes along with the corresponding
/// function's arguments.
fn remove_all_attrs<'a>(
    (attr_path, attr_arg): (&str, &str),
    func: &'a mut syn::ItemFn,
) -> (Vec<&'a syn::FnArg>, Vec<syn::Attribute>) {
    func.sig
        .inputs
        .iter_mut()
        .filter_map(|arg| {
            if let Some(attr) = remove_attr((attr_path, attr_arg), arg) {
                return Some((&*arg, attr));
            }
            None
        })
        .unzip()
}

/// Removes attribute `#[attr_path(attr_arg)]` from function's argument, if any.
fn remove_attr(
    (attr_path, attr_arg): (&str, &str),
    arg: &mut syn::FnArg,
) -> Option<syn::Attribute> {
    use itertools::{Either, Itertools as _};

    if let syn::FnArg::Typed(typed_arg) = arg {
        let attrs = mem::take(&mut typed_arg.attrs);

        let (mut other, mut removed): (Vec<_>, Vec<_>) = attrs.into_iter().partition_map(|attr| {
            if eq_path_and_arg((attr_path, attr_arg), &attr) {
                Either::Right(attr)
            } else {
                Either::Left(attr)
            }
        });

        if removed.len() == 1 {
            typed_arg.attrs = other;
            // Unwrapping is OK here, because `step_idents.len() == 1`.
            return Some(removed.pop().unwrap());
        } else {
            other.append(&mut removed);
            typed_arg.attrs = other;
        }
    }
    None
}

/// Compares attribute's path and argument.
fn eq_path_and_arg((attr_path, attr_arg): (&str, &str), attr: &syn::Attribute) -> bool {
    if let Ok(meta) = attr.parse_meta() {
        if let syn::Meta::List(meta_list) = meta {
            if meta_list.path.is_ident(attr_path) && meta_list.nested.len() == 1 {
                // Unwrapping is OK here, because `meta_list.nested.len() == 1`.
                if let syn::NestedMeta::Meta(m) = meta_list.nested.first().unwrap() {
                    return m.path().is_ident(attr_arg);
                }
            }
        }
    }
    false
}

/// Parses [`syn::Ident`] and [`syn::Type`] from the given [`syn::FnArg`].
fn parse_fn_arg(arg: &syn::FnArg) -> syn::Result<(&syn::Ident, &syn::Type)> {
    let arg = match arg {
        syn::FnArg::Typed(t) => t,
        _ => {
            return Err(syn::Error::new(
                arg.span(),
                "Expected regular argument, found `self`",
            ))
        }
    };

    let ident = match arg.pat.as_ref() {
        syn::Pat::Ident(i) => &i.ident,
        _ => return Err(syn::Error::new(arg.span(), "Expected ident")),
    };

    Ok((ident, arg.ty.as_ref()))
}

/// Parses type of a slice element from a second argument of the given function
/// signature.
fn parse_slice_from_second_arg(sig: &syn::Signature) -> Option<&syn::TypePath> {
    sig.inputs
        .iter()
        .nth(1)
        .and_then(|second_arg| match second_arg {
            syn::FnArg::Typed(typed_arg) => Some(typed_arg),
            _ => None,
        })
        .and_then(|typed_arg| match typed_arg.ty.as_ref() {
            syn::Type::Reference(r) => Some(r),
            _ => None,
        })
        .and_then(|ty_ref| match ty_ref.elem.as_ref() {
            syn::Type::Slice(s) => Some(s),
            _ => None,
        })
        .and_then(|slice| match slice.elem.as_ref() {
            syn::Type::Path(ty) => Some(ty),
            _ => None,
        })
}

/// Parses [`cucumber::World`] from arguments of the function signature.
///
/// [`cucumber::World`]: cucumber_rust::World
fn parse_world_from_args(sig: &syn::Signature) -> syn::Result<&syn::TypePath> {
    sig.inputs
        .first()
        .ok_or_else(|| sig.ident.span())
        .and_then(|first_arg| match first_arg {
            syn::FnArg::Typed(a) => Ok(a),
            _ => Err(first_arg.span()),
        })
        .and_then(|typed_arg| match typed_arg.ty.as_ref() {
            syn::Type::Reference(r) => Ok(r),
            _ => Err(typed_arg.span()),
        })
        .and_then(|world_ref| match world_ref.mutability {
            Some(_) => Ok(world_ref),
            None => Err(world_ref.span()),
        })
        .and_then(|world_mut_ref| match world_mut_ref.elem.as_ref() {
            syn::Type::Path(p) => Ok(p),
            _ => Err(world_mut_ref.span()),
        })
        .map_err(|span| {
            syn::Error::new(span, "First function argument expected to be `&mut World`")
        })
}

/// Converts [`syn::Lit`] to [`syn::LitStr`] if possible.
fn to_string_literal(l: syn::Lit) -> syn::Result<syn::LitStr> {
    match l {
        syn::Lit::Str(str) => Ok(str),
        _ => Err(syn::Error::new(l.span(), "Expected string literal")),
    }
}
