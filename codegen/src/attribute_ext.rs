// Copyright (c) 2020-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extended `#[given]`, `#[when]` and `#[then]` attribute macros with DataTable support.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;

/// Information about a DataTable parameter in a step function.
#[derive(Clone, Debug)]
pub struct DataTableParam {
    /// Name of the parameter.
    pub ident: syn::Ident,
    /// Whether it's optional (Option<DataTable>).
    pub is_optional: bool,
    /// Position in the function arguments.
    pub position: usize,
}

/// Detects DataTable parameters in a function signature.
pub fn detect_table_param(func: &syn::ItemFn) -> Option<DataTableParam> {
    func.sig
        .inputs
        .iter()
        .enumerate()
        .skip(1) // Skip world parameter
        .find_map(|(pos, arg)| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let ident = &pat_ident.ident;
                    
                    // Check if this is a DataTable or Option<DataTable>
                    if is_data_table_type(&pat_type.ty) {
                        return Some(DataTableParam {
                            ident: ident.clone(),
                            is_optional: is_option_data_table(&pat_type.ty),
                            position: pos,
                        });
                    }
                }
            }
            None
        })
}

/// Checks if a type is DataTable.
fn is_data_table_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "DataTable" || 
                       (segment.ident == "Option" && is_option_data_table(ty));
            }
            false
        }
        _ => false,
    }
}

/// Checks if a type is Option<DataTable>.
fn is_option_data_table(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        if let syn::Type::Path(inner_path) = inner_ty {
                            if let Some(inner_segment) = inner_path.path.segments.last() {
                                return inner_segment.ident == "DataTable";
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Generates code to inject a DataTable parameter.
pub fn generate_table_injection(
    table_param: &DataTableParam,
    func_name: &syn::Ident,
) -> TokenStream {
    let ident = &table_param.ident;
    
    if table_param.is_optional {
        // For Option<DataTable>, always provide Some or None
        quote! {
            let #ident = __cucumber_ctx.step.table.as_ref()
                .map(::cucumber::DataTable::from);
        }
    } else {
        // For required DataTable, panic if not present
        quote! {
            let #ident = __cucumber_ctx.step.table.as_ref()
                .map(::cucumber::DataTable::from)
                .expect(concat!(
                    "Step function `", 
                    stringify!(#func_name), 
                    "` requires a DataTable but none was provided in the feature file"
                ));
        }
    }
}

/// Modifies the function call to include DataTable parameter.
pub fn modify_function_call(
    original_call: TokenStream,
    table_param: Option<&DataTableParam>,
) -> TokenStream {
    if let Some(param) = table_param {
        let ident = &param.ident;
        // Insert the DataTable parameter in the correct position
        quote! {
            #original_call, #ident
        }
    } else {
        original_call
    }
}

/// Validates that DataTable parameters are used correctly.
pub fn validate_table_params(func: &syn::ItemFn) -> syn::Result<()> {
    let table_params: Vec<_> = func.sig
        .inputs
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(pos, arg)| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if is_data_table_type(&pat_type.ty) {
                    return Some((pos, arg));
                }
            }
            None
        })
        .collect();
    
    if table_params.len() > 1 {
        return Err(syn::Error::new(
            table_params[1].1.span(),
            "Only one DataTable parameter is allowed per step function",
        ));
    }
    
    // Check that DataTable comes after World and captures but before Step
    if let Some((pos, param)) = table_params.first() {
        // Find if there's a step parameter
        let step_param_pos = func.sig.inputs.iter().position(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    return pat_ident.ident == "step";
                }
            }
            false
        });
        
        if let Some(step_pos) = step_param_pos {
            if *pos > step_pos {
                return Err(syn::Error::new(
                    param.span(),
                    "DataTable parameter must come before the Step parameter",
                ));
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    
    #[test]
    fn test_detect_data_table() {
        let func: syn::ItemFn = parse_quote! {
            async fn test(world: &mut World, table: DataTable) {
                // function body
            }
        };
        
        let param = detect_table_param(&func);
        assert!(param.is_some());
        
        let param = param.unwrap();
        assert_eq!(param.ident, "table");
        assert!(!param.is_optional);
        assert_eq!(param.position, 1);
    }
    
    #[test]
    fn test_detect_optional_data_table() {
        let func: syn::ItemFn = parse_quote! {
            async fn test(world: &mut World, table: Option<DataTable>) {
                // function body
            }
        };
        
        let param = detect_table_param(&func);
        assert!(param.is_some());
        
        let param = param.unwrap();
        assert_eq!(param.ident, "table");
        assert!(param.is_optional);
    }
    
    #[test]
    fn test_no_data_table() {
        let func: syn::ItemFn = parse_quote! {
            async fn test(world: &mut World, name: String) {
                // function body
            }
        };
        
        let param = detect_table_param(&func);
        assert!(param.is_none());
    }
}