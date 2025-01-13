//! Derive macros for the service crate

use proc_macro::TokenStream;
use syn::parse_macro_input;
use tool::ServiceImpl;

mod tool;

/// Generate tools for the service crate, for example
///
/// ```rust
/// #[cydonia_service::service]
/// impl MyService {
///     /// My function
///     fn my_function(
///         // The first number
///         a: u64,
///         // The second number
///         b: u64,
///     ) -> u64 {
///         // ...
///     }
/// }
/// ```
///
/// generates
///
/// ```rust
/// impl MyService {
///     // My function
///     fn my_function(
///         // The first number
///         a: u64,
///         // The second number
///         b: u64,
///     ) -> u64 {
///         // ...
///     }
///
///     fn tools() -> Vec<Function> {
///         vec![Function {
///             name: "my_function".to_string(),
///             description: "My function".to_string(),
///             arguments: vec![
///                 Argument {
///                     name: "a".to_string(),
///                     description: "The first number".to_string(),
///                     type: "uint64".to_string(),
///                 },
///                 Argument {
///                     name: "b".to_string(),
///                     description: "The second number".to_string(),
///                     type: "uint64".to_string(),
///                 },
///             ],
///         }]
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn service(_: TokenStream, item: TokenStream) -> TokenStream {
    let service_impl = parse_macro_input!(item as ServiceImpl);
    service_impl.into_token_stream()
}
