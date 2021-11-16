// Copyright (c) 2020-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `#[given]`, `#[when]` and `#[then]` attribute macros implementation.

use std::mem;

use inflections::case::to_pascal_case;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::{self, Regex};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

/// Generates code of `#[given]`, `#[when]` and `#[then]` attribute macros
/// expansion.
pub(crate) fn step(
    attr_name: &'static str,
    args: TokenStream,
    input: TokenStream,
) -> syn::Result<TokenStream> {
    Step::parse(attr_name, args, input).and_then(Step::expand)
}

/// Parsed state (ready for code generation) of the attribute and the function
/// it's applied to.
#[derive(Clone, Debug)]
struct Step {
    /// Name of the attribute (`given`, `when` or `then`).
    attr_name: &'static str,

    /// Argument of the attribute.
    attr_arg: AttributeArgument,

    /// Function the attribute is applied to.
    func: syn::ItemFn,

    /// Name of the function argument representing a [`gherkin::Step`]
    /// reference.
    ///
    /// [`gherkin::Step`]: https://bit.ly/3j42hcd
    step_arg_name: Option<syn::Ident>,
}

impl Step {
    /// Parses [`Step`] definition from the attribute macro input.
    fn parse(
        attr_name: &'static str,
        attr: TokenStream,
        body: TokenStream,
    ) -> syn::Result<Self> {
        let attr_arg = syn::parse2::<AttributeArgument>(attr)?;
        let mut func = syn::parse2::<syn::ItemFn>(body)?;

        let step_arg_name = {
            let (arg_marked_as_step, _) =
                remove_all_attrs_if_needed("step", &mut func);

            match arg_marked_as_step.len() {
                0 => Ok(None),
                1 => {
                    let (ident, _) = parse_fn_arg(arg_marked_as_step[0])?;
                    Ok(Some(ident.clone()))
                }
                _ => Err(syn::Error::new(
                    arg_marked_as_step[1].span(),
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
            attr_name,
            attr_arg,
            func,
            step_arg_name,
        })
    }

    /// Expands generated code of this [`Step`] definition.
    fn expand(self) -> syn::Result<TokenStream> {
        let func = &self.func;
        let func_name = &func.sig.ident;

        let world = parse_world_from_args(&self.func.sig)?;
        let step_type = self.step_type();
        let (func_args, addon_parsing) =
            self.fn_arguments_and_additional_parsing()?;

        let step_matcher = self.attr_arg.regex_literal().value();
        let caller_name =
            format_ident!("__cucumber_{}_{}", self.attr_name, func_name);
        let awaiting = func.sig.asyncness.map(|_| quote! { .await });
        let unwrapping = (!self.returns_unit())
            .then(|| quote! { .unwrap_or_else(|e| panic!("{}", e)) });
        let step_caller = quote! {
            {
                #[automatically_derived]
                fn #caller_name<'w>(
                    __cucumber_world: &'w mut #world,
                    __cucumber_ctx: ::cucumber::step::Context,
                ) -> ::cucumber::codegen::LocalBoxFuture<'w, ()> {
                    let f = async move {
                        #addon_parsing
                        ::std::mem::drop(
                            #func_name(__cucumber_world, #func_args)
                                #awaiting
                                #unwrapping,
                        );
                    };
                    ::std::boxed::Box::pin(f)
                }

                let f: ::cucumber::Step<#world> = #caller_name;
                f
            }
        };

        Ok(quote! {
            #func

            #[automatically_derived]
            ::cucumber::codegen::submit!({
                // TODO: remove this, once `#![feature(more_qualified_paths)]`
                //       is stabilized.
                //       https://github.com/rust-lang/rust/issues/86935
                type StepAlias =
                    <#world as ::cucumber::codegen::WorldInventory>::#step_type;

                StepAlias {
                    loc: ::cucumber::step::Location {
                        path: ::std::file!(),
                        line: ::std::line!(),
                        column: ::std::column!(),
                    },
                    regex: {
                        let lazy: ::cucumber::codegen::LazyRegex = || {
                            static LAZY: ::cucumber::codegen::Lazy<
                                ::cucumber::codegen::Regex
                            > = ::cucumber::codegen::Lazy::new(|| {
                                ::cucumber::codegen::Regex::new(#step_matcher)
                                    .unwrap()
                            });
                            LAZY.clone()
                        };

                        lazy
                    },
                    func: {
                        // This hack exists, as `fn item` to `fn pointer`
                        // coercion can be done inside `const`, but not
                        // `const fn`.
                        const F: ::cucumber::Step<#world> = #step_caller;
                        F
                    },
                }
            });
        })
    }

    /// Indicates whether this [`Step::func`] return type is `()`.
    fn returns_unit(&self) -> bool {
        match &self.func.sig.output {
            syn::ReturnType::Default => true,
            syn::ReturnType::Type(_, ty) => {
                if let syn::Type::Tuple(syn::TypeTuple { elems, .. }) = &**ty {
                    elems.is_empty()
                } else {
                    false
                }
            }
        }
    }

    /// Generates code that prepares function's arguments basing on
    /// [`AttributeArgument`] and additional parsing if it's an
    /// [`AttributeArgument::Regex`].
    fn fn_arguments_and_additional_parsing(
        &self,
    ) -> syn::Result<(TokenStream, Option<TokenStream>)> {
        let is_regex = matches!(self.attr_arg, AttributeArgument::Regex(_));
        let func = &self.func;

        if is_regex {
            if let Some(elem_ty) = find_first_slice(&func.sig) {
                let addon_parsing = Some(quote! {
                    let __cucumber_matches = __cucumber_ctx
                        .matches
                        .iter()
                        .skip(1)
                        .enumerate()
                        .map(|(i, s)| {
                            s.parse::<#elem_ty>().unwrap_or_else(|e| panic!(
                                "Failed to parse element at {} '{}': {}",
                                i, s, e,
                            ))
                        })
                        .collect::<Vec<_>>();
                });
                let func_args = func
                    .sig
                    .inputs
                    .iter()
                    .skip(1)
                    .map(|arg| self.borrow_step_or_slice(arg))
                    .collect::<Result<TokenStream, _>>()?;

                Ok((func_args, addon_parsing))
            } else {
                #[allow(clippy::redundant_closure_for_method_calls)]
                let (idents, parsings): (Vec<_>, Vec<_>) =
                    itertools::process_results(
                        func.sig
                            .inputs
                            .iter()
                            .skip(1)
                            .map(|arg| self.arg_ident_and_parse_code(arg)),
                        |i| i.unzip(),
                    )?;

                let addon_parsing = Some(quote! {
                    let mut __cucumber_iter = __cucumber_ctx
                        .matches.iter()
                        .skip(1);
                    #( #parsings )*
                });
                let func_args = quote! {
                    #( #idents, )*
                };

                Ok((func_args, addon_parsing))
            }
        } else if self.step_arg_name.is_some() {
            Ok((
                quote! { ::std::borrow::Borrow::borrow(&__cucumber_ctx.step), },
                None,
            ))
        } else {
            Ok((TokenStream::default(), None))
        }
    }

    /// Composes a name of the `cucumber::codegen::WorldInventory` associated
    /// type to wire this [`Step`] with.
    fn step_type(&self) -> syn::Ident {
        format_ident!("{}", to_pascal_case(self.attr_name))
    }

    /// Returns [`syn::Ident`] and parsing code of the given function's
    /// argument.
    ///
    /// Function's argument type have to implement [`FromStr`].
    ///
    /// [`FromStr`]: std::str::FromStr
    /// [`syn::Ident`]: struct@syn::Ident
    fn arg_ident_and_parse_code<'a>(
        &self,
        arg: &'a syn::FnArg,
    ) -> syn::Result<(&'a syn::Ident, TokenStream)> {
        let (ident, ty) = parse_fn_arg(arg)?;

        let is_ctx_arg =
            self.step_arg_name.as_ref().map(|i| *i == *ident) == Some(true);

        let decl = if is_ctx_arg {
            quote! {
                let #ident =
                    ::std::borrow::Borrow::borrow(&__cucumber_ctx.step);
            }
        } else {
            let ty = if let syn::Type::Path(p) = ty {
                p
            } else {
                return Err(syn::Error::new(ty.span(), "Type path expected"));
            };

            let not_found_err = format!("{} not found", ident);
            let parsing_err = format!(
                "{} can not be parsed to {}",
                ident,
                ty.path
                    .segments
                    .last()
                    .ok_or_else(|| {
                        syn::Error::new(ty.path.span(), "Type path expected")
                    })?
                    .ident,
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

    /// Generates code that borrows [`gherkin::Step`] from context if the given
    /// `arg` matches `step_arg_name`, or else borrows parsed slice.
    ///
    /// [`gherkin::Step`]: https://bit.ly/3j42hcd
    fn borrow_step_or_slice(
        &self,
        arg: &syn::FnArg,
    ) -> syn::Result<TokenStream> {
        if let Some(name) = &self.step_arg_name {
            let (ident, _) = parse_fn_arg(arg)?;
            if name == ident {
                return Ok(quote! {
                    ::std::borrow::Borrow::borrow(&__cucumber_ctx.step),
                });
            }
        }

        Ok(quote! {
            __cucumber_matches.as_slice(),
        })
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
    /// Returns a [`syn::LitStr`] to construct a [`Regex`] with.
    ///
    /// [`syn::LitStr`]: struct@syn::LitStr
    fn regex_literal(&self) -> syn::LitStr {
        match self {
            Self::Regex(l) => l.clone(),
            Self::Literal(l) => syn::LitStr::new(
                &format!("^{}$", regex::escape(&l.value())),
                l.span(),
            ),
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

                    drop(Regex::new(str_lit.value().as_str()).map_err(
                        |e| {
                            syn::Error::new(
                                str_lit.span(),
                                format!("Invalid regex: {}", e),
                            )
                        },
                    )?);

                    Ok(Self::Regex(str_lit))
                } else {
                    Err(syn::Error::new(arg.span(), "Expected regex argument"))
                }
            }

            syn::NestedMeta::Lit(l) => Ok(Self::Literal(to_string_literal(l)?)),

            syn::NestedMeta::Meta(_) => Err(syn::Error::new(
                arg.span(),
                "Expected string literal or regex argument",
            )),
        }
    }
}

/// Removes all `#[attr_arg]` attributes from the given function signature and
/// returns these attributes along with the corresponding function's arguments
/// in case there are no more `#[given]`, `#[when]` or `#[then]` attributes.
fn remove_all_attrs_if_needed<'a>(
    attr_arg: &str,
    func: &'a mut syn::ItemFn,
) -> (Vec<&'a syn::FnArg>, Vec<syn::Attribute>) {
    let has_other_step_arguments = func.attrs.iter().any(|attr| {
        attr.path
            .segments
            .last()
            .map(|segment| {
                ["given", "when", "then"]
                    .iter()
                    .any(|step| segment.ident == step)
            })
            .unwrap_or_default()
    });

    func.sig
        .inputs
        .iter_mut()
        .filter_map(|arg| {
            if has_other_step_arguments {
                find_attr(attr_arg, arg)
            } else {
                remove_attr(attr_arg, arg)
            }
            .map(move |attr| (&*arg, attr))
        })
        .unzip()
}

/// Finds attribute `#[attr_arg]` from function's argument, if any.
fn find_attr(attr_arg: &str, arg: &mut syn::FnArg) -> Option<syn::Attribute> {
    if let syn::FnArg::Typed(typed_arg) = arg {
        typed_arg
            .attrs
            .iter()
            .find(|attr| {
                attr.path
                    .get_ident()
                    .map(|ident| ident == attr_arg)
                    .unwrap_or_default()
            })
            .cloned()
    } else {
        None
    }
}

/// Removes attribute `#[attr_arg]` from function's argument, if any.
fn remove_attr(attr_arg: &str, arg: &mut syn::FnArg) -> Option<syn::Attribute> {
    use itertools::{Either, Itertools as _};

    if let syn::FnArg::Typed(typed_arg) = arg {
        let attrs = mem::take(&mut typed_arg.attrs);

        let (mut other, mut removed): (Vec<_>, Vec<_>) =
            attrs.into_iter().partition_map(|attr| {
                if let Some(ident) = attr.path.get_ident() {
                    if ident == attr_arg {
                        return Either::Right(attr);
                    }
                }
                Either::Left(attr)
            });

        if removed.len() == 1 {
            typed_arg.attrs = other;
            return removed.pop();
        }
        other.append(&mut removed);
        typed_arg.attrs = other;
    }
    None
}

/// Parses [`syn::Ident`] and [`syn::Type`] from the given [`syn::FnArg`].
///
/// [`syn::Ident`]: struct@syn::Ident
fn parse_fn_arg(arg: &syn::FnArg) -> syn::Result<(&syn::Ident, &syn::Type)> {
    let arg = match arg {
        syn::FnArg::Typed(t) => t,
        syn::FnArg::Receiver(_) => {
            return Err(syn::Error::new(
                arg.span(),
                "Expected regular argument, found `self`",
            ))
        }
    };

    let ident = if let syn::Pat::Ident(i) = arg.pat.as_ref() {
        &i.ident
    } else {
        return Err(syn::Error::new(arg.span(), "Expected ident"));
    };

    Ok((ident, arg.ty.as_ref()))
}

/// Parses type of a first slice element of the given function signature.
fn find_first_slice(sig: &syn::Signature) -> Option<&syn::TypePath> {
    sig.inputs.iter().find_map(|arg| {
        match arg {
            syn::FnArg::Typed(typed_arg) => Some(typed_arg),
            syn::FnArg::Receiver(_) => None,
        }
        .and_then(|typed_arg| {
            if let syn::Type::Reference(r) = typed_arg.ty.as_ref() {
                Some(r)
            } else {
                None
            }
            .and_then(|ty_ref| {
                if let syn::Type::Slice(s) = ty_ref.elem.as_ref() {
                    Some(s)
                } else {
                    None
                }
                .and_then(|slice| {
                    if let syn::Type::Path(ty) = slice.elem.as_ref() {
                        Some(ty)
                    } else {
                        None
                    }
                })
            })
        })
    })
}

/// Parses `cucumber::World` from arguments of the function signature.
fn parse_world_from_args(sig: &syn::Signature) -> syn::Result<&syn::TypePath> {
    sig.inputs
        .first()
        .ok_or_else(|| sig.ident.span())
        .and_then(|first_arg| match first_arg {
            syn::FnArg::Typed(a) => Ok(a),
            syn::FnArg::Receiver(_) => Err(first_arg.span()),
        })
        .and_then(|typed_arg| {
            if let syn::Type::Reference(r) = typed_arg.ty.as_ref() {
                Ok(r)
            } else {
                Err(typed_arg.span())
            }
        })
        .and_then(|world_ref| match world_ref.mutability {
            Some(_) => Ok(world_ref),
            None => Err(world_ref.span()),
        })
        .and_then(|world_mut_ref| {
            if let syn::Type::Path(p) = world_mut_ref.elem.as_ref() {
                Ok(p)
            } else {
                Err(world_mut_ref.span())
            }
        })
        .map_err(|span| {
            syn::Error::new(
                span,
                "First function argument expected to be `&mut World`",
            )
        })
}

/// Converts [`syn::Lit`] to [`syn::LitStr`] if possible.
///
/// [`syn::Lit`]: enum@syn::Lit
/// [`syn::LitStr`]: struct@syn::LitStr
fn to_string_literal(l: syn::Lit) -> syn::Result<syn::LitStr> {
    if let syn::Lit::Str(str) = l {
        Ok(str)
    } else {
        Err(syn::Error::new(l.span(), "Expected string literal"))
    }
}
