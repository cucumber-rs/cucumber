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
use quote::quote;

/// Information about a DataTable parameter in a step function.
#[derive(Clone, Debug)]
pub(crate) struct DataTableParam {
    /// Name of the parameter.
    pub ident: syn::Ident,
    /// Whether it's optional (Option<DataTable>).
    pub is_optional: bool,
    /// Position in the function arguments.
    #[allow(dead_code)]
    pub position: usize,
}

/// Detects DataTable parameters in a function signature.
pub(crate) fn detect_table_param(func: &syn::ItemFn) -> Option<DataTableParam> {
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

/// Checks if a function argument is a DataTable type.
pub(crate) fn is_data_table_type_from_arg(arg: &syn::FnArg) -> bool {
    if let syn::FnArg::Typed(pat_type) = arg {
        is_data_table_type(&pat_type.ty)
    } else {
        false
    }
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
pub(crate) fn is_option_data_table(ty: &syn::Type) -> bool {
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
pub(crate) fn generate_table_injection(
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