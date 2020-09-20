// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

mod serialize;
use serialize::*;

use proc_macro2::Span;
use proc_macro_crate::crate_name;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::quote;
use syn::*;

#[proc_macro_derive(CanonicalSerialize)]
pub fn derive_canonical_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    proc_macro::TokenStream::from(impl_canonical_serialize(&ast))
}

#[proc_macro_derive(CanonicalDeserialize)]
pub fn derive_canonical_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    proc_macro::TokenStream::from(impl_canonical_deserialize(&ast))
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn test_with_metrics(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match parse::<ItemFn>(item.clone()) {
        Ok(function) => {
            fn generate_test_function(function: ItemFn, crate_name: Ident) -> proc_macro::TokenStream {
                let name = &function.sig.ident;
                let statements = function.block.stmts;
                (quote! {
                    // Generates a new test with Prometheus registry checks, and enforces
                    // that the test runs serially with other tests that use metrics.
                    #[test]
                    #[serial]
                    fn #name() {
                        // Initialize Prometheus once in the test environment.
                        #crate_name::testing::initialize_prometheus_for_testing();
                        // Check that all metrics are 0 or empty.
                        assert_eq!(0, #crate_name::Metrics::get_connected_peers());
                        // Run the test logic.
                        #(#statements)*
                        // Check that all metrics are reset to 0 or empty (for next test).
                        assert_eq!(0, Metrics::get_connected_peers());
                    }
                })
                .into()
            }
            let name = crate_name("snarkos-metrics").unwrap_or("crate".to_string());
            let crate_name = Ident::new(&name, Span::call_site());
            generate_test_function(function, crate_name)
        }
        _ => abort_call_site!("test_with_metrics only works on functions"),
    }
}
