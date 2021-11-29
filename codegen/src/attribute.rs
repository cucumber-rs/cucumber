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

use std::{iter, mem};

use cucumber_expressions::{Expression, Parameter, SingleExpression, Spanned};
use inflections::case::to_pascal_case;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::{self, Regex};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
};

/// TODO
const DEFAULT_EXPRESSION_PARS: [&str; 5] =
    ["int", "float", "word", "string", ""];

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

        let regex = self.generate_regex()?;

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
                // TODO: Remove this, once `#![feature(more_qualified_paths)]`
                //       is stabilized:
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
                        // This hack exists, as `fn item` to `fn pointer`
                        // coercion can be done inside `const`, but not
                        // `const fn`.
                        let lazy: ::cucumber::codegen::LazyRegex = || {
                            static LAZY: ::cucumber::codegen::Lazy<
                                ::cucumber::codegen::Regex
                            > = ::cucumber::codegen::Lazy::new(|| {
                                #regex
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
        let is_regex_or_expr = matches!(
            self.attr_arg,
            AttributeArgument::Regex(_) | AttributeArgument::Expression(_),
        );
        let func = &self.func;

        if is_regex_or_expr {
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

    /// TODO
    fn generate_regex(&self) -> syn::Result<TokenStream> {
        match &self.attr_arg {
            AttributeArgument::Literal(l) => {
                let l = syn::LitStr::new(
                    &format!("^{}$", regex::escape(&l.value())),
                    l.span(),
                );
                Ok(quote! { ::cucumber::codegen::Regex::new(#l).unwrap() })
            }
            AttributeArgument::Regex(re) => {
                drop(Regex::new(re.value().as_str()).map_err(|e| {
                    syn::Error::new(re.span(), format!("Invalid regex: {}", e))
                })?);

                Ok(quote! { ::cucumber::codegen::Regex::new(#re).unwrap() })
            }
            AttributeArgument::Expression(expr) => {
                self.generate_expression_regex(expr)
            }
        }
    }

    /// TODO
    fn generate_expression_regex(
        &self,
        expr: &syn::LitStr,
    ) -> syn::Result<TokenStream> {
        let expr = expr.value();

        let parameters =
            Parameters::new(&expr, &self.func, self.step_arg_name.as_ref())?;
        let provider_impl =
            parameters.generate_provider_impl(&format_ident!("Provider"));
        let assertions = parameters.generate_assertions();

        Ok(quote! {{
            #assertions

            #[automatically_derived]
            #[derive(Clone, Copy)]
            struct Provider;

            #provider_impl

            // This should never fail because:
            // 1. We checked AST correction with `Expression::parse()`;
            // 2. Custom `Parameter::REGEX`es are correct due to derive macro;
            // 3. All parameter names are equal to the corresponding fn
            //    arguments, so we shouldn't see `UnknownParameterError`.
            ::cucumber::codegen::Expression::regex_with_parameters(
                #expr,
                Provider,
            )
            .unwrap_or_else(|e| {
                panic!("Cucumber expression failed: {}", e)
            })
        }})
    }
}

/// TODO
struct ParameterProvider<'p> {
    /// TODO
    ast: Parameter<Spanned<'p>>,

    /// TODO
    ty: syn::Type,
}

/// TODO
struct Parameters<'p>(Vec<ParameterProvider<'p>>);

impl<'p> Parameters<'p> {
    /// TODO
    fn new(
        expr: &'p str,
        func: &syn::ItemFn,
        step: Option<&syn::Ident>,
    ) -> syn::Result<Self> {
        let expr_ast = Expression::parse(expr).map_err(|e| {
            syn::Error::new(
                expr.span(),
                format!("Incorrect cucumber expression: {}", e),
            )
        })?;

        let parameter_types = func
            .sig
            .inputs
            .iter()
            .skip(1)
            .filter_map(|arg| {
                let (ident, ty) = match parse_fn_arg(arg) {
                    Ok(res) => res,
                    Err(err) => return Some(Err(err)),
                };
                let is_step = step.map(|s| s == ident).unwrap_or_default();
                (!is_step).then(|| Ok(ty))
            })
            .collect::<syn::Result<Vec<_>>>()?;

        expr_ast
            .0
            .into_iter()
            .filter_map(|e| match e {
                SingleExpression::Parameter(par) => Some(par),
                SingleExpression::Alternation(_)
                | SingleExpression::Optional(_)
                | SingleExpression::Text(_)
                | SingleExpression::Whitespaces(_) => None,
            })
            .zip(
                parameter_types
                    .into_iter()
                    .map(Some)
                    .chain(iter::repeat(None)),
            )
            .filter_map(|(ast, ty)| {
                if DEFAULT_EXPRESSION_PARS.iter().any(|s| s == &**ast) {
                    // If parameter is default, it's ok if there is no type
                    // corresponding to it, as we know it's regex.
                    ty.cloned().map(|ty| Ok(ParameterProvider { ast, ty }))
                } else if let Some(ty) = ty.cloned() {
                    Some(Ok(ParameterProvider { ast, ty }))
                } else {
                    Some(Err(syn::Error::new(
                        func.span(),
                        format!(
                            "Function argument corresponding to the `{p}` \
                             parameter isn't found. Consider adding \
                             argument which implements `Parameter` with \
                             `Parameter::NAME == {p}`.",
                            p = *ast,
                        ),
                    )))
                }
            })
            .collect::<syn::Result<Vec<_>>>()
            .map(Self)
    }

    /// TODO
    fn generate_assertions(&self) -> TokenStream {
        self.0
            .iter()
            .map(|par| {
                let name = par.ast.0.fragment();
                let ty = &par.ty;

                if DEFAULT_EXPRESSION_PARS.contains(name) {
                    let trait_with_hint = format_ident!(
                        "UseParameterNameInsteadOf{}",
                        to_pascal_case(name),
                    );
                    quote! {
                        // In case we encounter default parameter, we should
                        // assert that corresponding type __doesn't__ implement
                        // Parameter trait.
                        #[automatically_derived]
                        const _: fn() = || {
                            // Generic trait with a blanket impl over `()` for
                            // all types.
                            #[automatically_derived]
                            trait #trait_with_hint<A> {
                                fn method() {}
                            }

                            #[automatically_derived]
                            impl<T: ?Sized> #trait_with_hint<()> for T {}

                            // Used for the specialized impl when Parameter is
                            // implemented.
                            #[allow(dead_code)]
                            #[automatically_derived]
                            struct Invalid;

                            #[automatically_derived]
                            impl<T: ?Sized + ::cucumber::Parameter>
                                #trait_with_hint<Invalid> for T {}

                            // If there is only one specialized trait impl, type
                            // inference with `_` can be resolved and this can
                            // compile. Fails to compile if `#ty` implements
                            // `ParameterShouldNotBeImpled<Invalid>`.
                            let _: fn() = <#ty as #trait_with_hint<_>>::method;
                        };
                    }
                } else {
                    quote! {
                        // In case we encounter custom parameter, we should
                        // assert that corresponding type implements Parameter
                        // and has right NAME.
                        #[allow(unknown_lints, eq_op)]
                        #[automatically_derived]
                        const _: [
                            ();
                            0 - !{
                                const ASSERT: bool =
                                    ::cucumber::codegen::str_eq(
                                        <#ty as ::cucumber::Parameter>::NAME,
                                        #name,
                                    );
                                ASSERT
                            } as usize
                        ] = [];
                    }
                }
            })
            .collect()
    }

    /// TODO
    fn generate_provider_impl(&self, ident: &syn::Ident) -> TokenStream {
        let (custom_par, custom_par_ty): (Vec<_>, Vec<_>) = self
            .0
            .iter()
            .filter_map(|par| {
                let name = par.ast.0.fragment();
                (!DEFAULT_EXPRESSION_PARS.contains(name))
                    .then(|| (*name, &par.ty))
            })
            .unzip();

        quote! {
                #[automatically_derived]
                impl<'s> ::cucumber::codegen::Provider<
                    ::cucumber::codegen::Spanned<'s>
                > for #ident {
                    type Item = char;
                    type Value = &'static str;

                    fn get(&self, input: &::cucumber::codegen::Spanned<'s>) ->
                        ::std::option::Option<Self::Value>
                    {
                        #( if *input.fragment() == #custom_par {
                            ::std::option::Option::Some(
                                <#custom_par_ty as ::cucumber::Parameter>::REGEX
                            )
                        } else )*
                        {
                            ::std::option::Option::None
                        }
                    }
                }
        }
    }
}

/// Argument of the attribute macro.
#[derive(Clone, Debug)]
enum AttributeArgument {
    /// `#[step("literal")]` case.
    Literal(syn::LitStr),

    /// `#[step(regex = "regex")]` case.
    Regex(syn::LitStr),

    /// `#[step(expr = "cucumber-expression")]` case.
    Expression(syn::LitStr),
}

impl Parse for AttributeArgument {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let arg = input.parse::<syn::NestedMeta>()?;
        match arg {
            syn::NestedMeta::Meta(syn::Meta::NameValue(arg)) => {
                match arg.path.get_ident() {
                    Some(i) if i == "regex" => {
                        Ok(Self::Regex(to_string_literal(arg.lit)?))
                    }
                    Some(i) if i == "expr" => {
                        Ok(Self::Expression(to_string_literal(arg.lit)?))
                    }
                    _ => Err(syn::Error::new(
                        arg.span(),
                        "Expected `regex` or `expr` argument",
                    )),
                }
            }

            syn::NestedMeta::Lit(l) => Ok(Self::Literal(to_string_literal(l)?)),

            syn::NestedMeta::Meta(_) => Err(syn::Error::new(
                arg.span(),
                "Expected string literal, `regex` or `expr` argument",
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
