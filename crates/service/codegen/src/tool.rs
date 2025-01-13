//! Tool generation for the services

use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    GenericArgument, ImplItem, ItemImpl, PathArguments, Result, Type,
};

/// Structure to hold the parsed service implementation
pub struct ServiceImpl {
    impl_block: ItemImpl,
    functions: Vec<proc_macro2::TokenStream>,
}

impl Parse for ServiceImpl {
    fn parse(input: ParseStream) -> Result<Self> {
        let impl_block: ItemImpl = input.parse()?;
        let mut functions = Vec::new();

        // Collect all documented methods
        for item in &impl_block.items {
            // Skip non-function items
            let ImplItem::Fn(method) = item else {
                continue;
            };

            // Description of the method is required.
            let Some(doc) = method.attrs.iter().find(|attr| attr.path().is_ident("doc")) else {
                panic!("No doc found for method {}", method.sig.ident);
            };

            let description = doc.to_token_stream().to_string();
            let name = method.sig.ident.to_string();
            let mut arguments = Vec::new();
            for param in &method.sig.inputs {
                if let syn::FnArg::Typed(pat_type) = param {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        let param_name = pat_ident.ident.to_string();
                        let param_type = validate_param(param)?;

                        // Get parameter description from doc comment if available
                        let param_desc = if let Some(doc) = pat_type
                            .attrs
                            .iter()
                            .find(|attr| attr.path().is_ident("doc"))
                        {
                            doc.to_token_stream().to_string()
                        } else {
                            format!("Parameter {}", param_name)
                        };

                        arguments.push(quote! {
                            Argument {
                                name: #param_name.to_string(),
                                description: #param_desc.to_string(),
                                ty: #param_type.to_string(),
                            }
                        });
                    }
                }
            }

            functions.push(quote! {
                    Function {
                        name: #name.to_string(),
                        description: #description.to_string(),
                        arguments: vec![
                        #(#arguments),*
                    ],
                }
            });
        }

        Ok(ServiceImpl {
            impl_block,
            functions,
        })
    }
}

impl ServiceImpl {
    /// Convert the parsed service implementation into a TokenStream
    pub fn into_token_stream(self) -> proc_macro::TokenStream {
        let ty = &self.impl_block.self_ty;
        let functions = self.functions;
        let impl_block = self.impl_block.clone();

        quote! {
            #impl_block

            impl #ty {
                /// Generate tools for the service implementation
                pub fn tools() -> Vec<cydonia_service::Function> {
                    vec![
                        #(#functions),*
                    ]
                }
            }
        }
        .into_token_stream()
        .into()
    }
}

// 1. If it is and option, support the inner type of the option
// 2. tuple is not supported
// 3. `Vec<T>` or `slice` (e.g. `[u8]`) should be regarded as `Vec<T>`
// 4. other generic types are not supported, panic when unsupported
// function argument is found.
fn validate_param(param: &syn::FnArg) -> Result<String> {
    let syn::FnArg::Typed(pat_type) = param else {
        return Err(syn::Error::new_spanned(
            param,
            "Self parameters are not supported",
        ));
    };

    match &*pat_type.ty {
        Type::Path(type_path) => {
            let segment = type_path
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(type_path, "Empty type path"))?;

            // Handle Option<T>
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return validate_inner_type(inner_type);
                    }
                }
                return Err(syn::Error::new_spanned(segment, "Invalid Option type"));
            }

            // Handle Vec<T> or similar collection types
            if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return validate_inner_type(inner_type);
                    }
                }
                return Err(syn::Error::new_spanned(segment, "Invalid Vec type"));
            }

            // Handle basic types
            match segment.ident.to_string().as_str() {
                "String" | "str" | "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" => {
                    Ok(segment.ident.to_string())
                }
                _ => Err(syn::Error::new_spanned(segment, "Unsupported type")),
            }
        }
        Type::Reference(type_ref) => {
            // Handle slices like &[T]
            if let Type::Slice(slice) = &*type_ref.elem {
                return validate_inner_type(&slice.elem).map(|t| format!("Vec<{}>", t));
            }

            // Handle string slices
            if let Type::Path(type_path) = &*type_ref.elem {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "str" {
                        return Ok("String".to_string());
                    }
                }
            }

            validate_inner_type(&type_ref.elem)
        }
        Type::Slice(slice) => {
            // Handle direct slice types [T]
            validate_inner_type(&slice.elem).map(|t| format!("Vec<{}>", t))
        }
        Type::Tuple(_) => Err(syn::Error::new_spanned(
            pat_type,
            "Tuple types are not supported",
        )),
        _ => Err(syn::Error::new_spanned(pat_type, "Unsupported type")),
    }
}

fn validate_inner_type(ty: &Type) -> Result<String> {
    let Type::Path(type_path) = ty else {
        return Err(syn::Error::new_spanned(ty, "Unsupported inner type"));
    };

    let segment = type_path
        .path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(type_path, "Empty type path"))?;

    match segment.ident.to_string().as_str() {
        "String" | "str" | "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8"
        | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" => Ok(segment.ident.to_string()),
        _ => Err(syn::Error::new_spanned(segment, "Unsupported inner type")),
    }
}
