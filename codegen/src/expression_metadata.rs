//! Original-expression and built-in capture metadata expansion.

use std::collections::HashMap;

use cucumber_expressions::{Expression, SingleExpression};
use proc_macro2::TokenStream;
use quote::quote;

/// Preserves actual custom type descriptors when every parameter opts in.
pub(crate) fn options(expression: &syn::LitStr,
    runtime_parameter: fn(&str) -> Option<&'static str>, custom: &[(String, syn::Type)])
    -> syn::Result<TokenStream>
{
    if custom.is_empty() { return builtin_options(expression, runtime_parameter); }
    let source = expression.value();
    let parsed = Expression::parse(&source).map_err(|error|
        syn::Error::new(expression.span(), error.to_string()))?;
    let mut insertions = Vec::new();
    for name in parsed.0.iter().filter_map(|item| match item {
        SingleExpression::Parameter(parameter) => Some(parameter.input.fragment().to_string()),
        _ => None,
    }) {
        let (pattern, structural, description) = if let Some(pattern) = runtime_parameter(&name) {
            let structural = if name == "string" {
                r#""([^"\\]*(\\[^\n\r\u{2028}\u{2029}][^"\\]*)*)"|'([^'\\]*(\\[^\n\r\u{2028}\u{2029}][^'\\]*)*)'"#
            } else { pattern };
            let (sources, snippets, prefer) = descriptor(&name).ok_or_else(||
                syn::Error::new(expression.span(), "unknown builtin metadata descriptor"))?;
            (quote! { #pattern }, quote! { #structural }, quote! {
                ::cucumber::support_code::ParameterDescriptor {
                    name: #name.to_owned(), regular_expressions: ::std::vec![ #( #sources.to_owned() ),* ],
                    prefer_for_regular_expression_match: #prefer, use_for_snippets: #snippets,
                }
            })
        } else {
            // The existing provider resolves the first custom entry with this
            // name. Retain that actual resolution rather than using a later type.
            let (_, ty) = custom.iter().find(|(candidate, _)| *candidate == name).ok_or_else(||
                syn::Error::new(expression.span(), "custom metadata type is unavailable"))?;
            (quote! { <#ty as ::cucumber::Parameter>::REGEX },
             quote! { <#ty as ::cucumber::Parameter>::REGEX }, quote! {
                {
                    let Some(descriptor) = <#ty as ::cucumber::Parameter>::metadata() else {
                        return ::cucumber::step::DefinitionOptions::new();
                    };
                    ::std::assert_eq!(descriptor.name, <#ty as ::cucumber::Parameter>::NAME,
                        "custom parameter metadata name differs from actual type");
                    ::std::assert_eq!(descriptor.regular_expressions,
                        ::std::vec![<#ty as ::cucumber::Parameter>::REGEX.to_owned()],
                        "custom parameter metadata regex differs from actual type");
                    descriptor
                }
            })
        };
        insertions.push(quote! {
            runtime.entry(#name.to_owned()).or_insert_with(|| (#pattern).to_owned());
            captures.entry(#name.to_owned()).or_insert_with(|| ::std::format!("({})", #structural));
            descriptors.push(#description);
        });
    }
    Ok(quote! {
        {
            let mut runtime = ::std::collections::HashMap::<::std::string::String, ::std::string::String>::new();
            let mut captures = ::std::collections::HashMap::<::std::string::String, ::std::string::String>::new();
            let mut descriptors = ::std::vec::Vec::new();
            #( #insertions )*
            let runtime_regex = ::cucumber::codegen::Expression::regex_with_parameters(#source, &runtime)
                .expect("actual custom parameter matcher must be valid");
            let capture_regex = ::cucumber::codegen::Expression::regex_with_parameters(#source, &captures)
                .expect("custom metadata capture matcher must be valid");
            ::cucumber::step::DefinitionOptions::new().expression(
                ::cucumber::support_code::ExpressionMetadata::from_generated(
                    #source.to_owned(), runtime_regex.as_str(), capture_regex.as_str(), descriptors,
                ).expect("custom expression metadata layout must be valid")
            )
        }
    })
}

/// Builds metadata from the same expression AST and builtin runtime patterns.
/// Custom types without complete descriptors retain their existing raw report.
pub(crate) fn builtin_options(expression: &syn::LitStr,
    runtime_parameter: fn(&str) -> Option<&'static str>) -> syn::Result<TokenStream>
{
    let source = expression.value();
    let parsed = Expression::parse(&source).map_err(|error|
        syn::Error::new(expression.span(), error.to_string()))?;
    let names = parsed.0.iter().filter_map(|item| match item {
        SingleExpression::Parameter(parameter) => Some(parameter.input.fragment().to_string()),
        _ => None,
    }).collect::<Vec<_>>();
    let mut runtime = HashMap::<String, String>::new();
    let mut captures = HashMap::<String, String>::new();
    let mut descriptors = Vec::new();
    for name in names {
        let Some(pattern) = runtime_parameter(&name) else {
            return Ok(quote! { ::cucumber::step::DefinitionOptions::new() });
        };
        runtime.insert(name.clone(), pattern.to_owned());
        // The official string type has nested escape captures. Handler parsing
        // intentionally keeps its existing flatter layout; metadata preserves
        // the actual pinned structural layout and validates observed ranges.
        let structural = if name == "string" {
            r#""([^"\\]*(\\[^\n\r\u{2028}\u{2029}][^"\\]*)*)"|'([^'\\]*(\\[^\n\r\u{2028}\u{2029}][^'\\]*)*)'"#
        } else { pattern };
        captures.insert(name.clone(), format!("({structural})"));
        let (sources, snippets, prefer) = descriptor(&name)
            .ok_or_else(|| syn::Error::new(expression.span(), "unknown builtin metadata descriptor"))?;
        descriptors.push(quote! {
            ::cucumber::support_code::ParameterDescriptor {
                name: #name.to_owned(),
                regular_expressions: ::std::vec![ #( #sources.to_owned() ),* ],
                prefer_for_regular_expression_match: #prefer,
                use_for_snippets: #snippets,
            }
        });
    }
    let runtime_regex = Expression::regex_with_parameters(&source, &runtime)
        .map_err(|error| syn::Error::new(expression.span(), error.to_string()))?;
    let capture_regex = Expression::regex_with_parameters(&source, &captures)
        .map_err(|error| syn::Error::new(expression.span(), error.to_string()))?;
    let (runtime_pattern, capture_pattern) = (runtime_regex.as_str(), capture_regex.as_str());
    Ok(quote! {
        ::cucumber::step::DefinitionOptions::new().expression(
            ::cucumber::support_code::ExpressionMetadata::from_generated(
                #source.to_owned(), #runtime_pattern, #capture_pattern,
                ::std::vec![ #( #descriptors ),* ],
            ).expect("macro-generated expression capture layout must be valid")
        )
    })
}

/// Exact public descriptor vocabulary of cucumber-expressions 20.1.0.
fn descriptor(name: &str) -> Option<(Vec<&'static str>, bool, bool)> {
    const INTEGER: &[&str] = &[r"-?\d+", r"\d+"];
    const FLOAT: &str = r"[-+]?(?:\d+(?:\.\d+)?|\.\d+)(?:[E][+-]?\d+)?";
    const STRING: &str = r#""([^"\\]*(\\.[^"\\]*)*)"|'([^'\\]*(\\.[^'\\]*)*)'"#;
    Some(match name {
        "int" => (INTEGER.to_vec(), true, true),
        "float" => (vec![FLOAT], true, false),
        "word" => (vec![r"[^\s]+"], false, false),
        "string" => (vec![STRING], true, false),
        "" => (vec![".*"], false, true),
        "double" | "bigdecimal" => (vec![FLOAT], false, false),
        "byte" | "short" | "long" | "biginteger" => (INTEGER.to_vec(), false, false),
        _ => return None,
    })
}
