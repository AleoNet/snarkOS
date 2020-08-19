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

use proc_macro2::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Index, Type};

use quote::{quote, ToTokens};

#[proc_macro_derive(CanonicalSerialize)]
pub fn derive_canonical_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    proc_macro::TokenStream::from(impl_canonical_serialize(&ast))
}

fn impl_serialize_field(
    serialize_body: &mut Vec<TokenStream>,
    serialized_size_body: &mut Vec<TokenStream>,
    serialize_uncompressed_body: &mut Vec<TokenStream>,
    uncompressed_size_body: &mut Vec<TokenStream>,
    idents: &mut Vec<Box<dyn ToTokens>>,
    ty: &Type,
) {
    // Check if type is a tuple.
    match ty {
        Type::Tuple(tuple) => {
            for (i, elem_ty) in tuple.elems.iter().enumerate() {
                let index = Index::from(i);
                idents.push(Box::new(index));
                impl_serialize_field(
                    serialize_body,
                    serialized_size_body,
                    serialize_uncompressed_body,
                    uncompressed_size_body,
                    idents,
                    elem_ty,
                );
                idents.pop();
            }
        }
        _ => {
            serialize_body.push(quote! { CanonicalSerialize::serialize(&self.#(#idents).*, writer)?; });
            serialized_size_body.push(quote! { size += CanonicalSerialize::serialized_size(&self.#(#idents).*); });
            serialize_uncompressed_body
                .push(quote! { CanonicalSerialize::serialize_uncompressed(&self.#(#idents).*, writer)?; });
            uncompressed_size_body.push(quote! { size += CanonicalSerialize::uncompressed_size(&self.#(#idents).*); });
        }
    }
}

fn impl_canonical_serialize(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let mut serialize_body = Vec::<TokenStream>::new();
    let mut serialized_size_body = Vec::<TokenStream>::new();
    let mut serialize_uncompressed_body = Vec::<TokenStream>::new();
    let mut uncompressed_size_body = Vec::<TokenStream>::new();

    match ast.data {
        Data::Struct(ref data_struct) => {
            for (i, field) in data_struct.fields.iter().enumerate() {
                let mut idents = Vec::<Box<dyn ToTokens>>::new();
                match field.ident {
                    None => {
                        let index = Index::from(i);
                        idents.push(Box::new(index));
                    }
                    Some(ref ident) => {
                        idents.push(Box::new(ident.clone()));
                    }
                }

                impl_serialize_field(
                    &mut serialize_body,
                    &mut serialized_size_body,
                    &mut serialize_uncompressed_body,
                    &mut uncompressed_size_body,
                    &mut idents,
                    &field.ty,
                );
            }
        }
        _ => panic!("Serialize can only be derived for structs, {} is not a struct", name),
    };

    let gen = quote! {
        impl #impl_generics CanonicalSerialize for #name #ty_generics #where_clause {
            #[allow(unused_mut, unused_variables)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), SerializationError> {
                #(#serialize_body)*
                Ok(())
            }
            #[allow(unused_mut, unused_variables)]
            fn serialized_size(&self) -> usize {
                let mut size = 0;
                #(#serialized_size_body)*
                size
            }
            #[allow(unused_mut, unused_variables)]
            fn serialize_uncompressed<W: Write>(&self, writer: &mut W) -> Result<(), SerializationError> {
                #(#serialize_uncompressed_body)*
                Ok(())
            }
            #[allow(unused_mut, unused_variables)]
            fn uncompressed_size(&self) -> usize {
                let mut size = 0;
                #(#uncompressed_size_body)*
                size
            }
        }
    };
    gen
}

#[proc_macro_derive(CanonicalDeserialize)]
pub fn derive_canonical_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    proc_macro::TokenStream::from(impl_canonical_deserialize(&ast))
}

/// Returns two TokenStreams, one for the compressed deserialize, one for the
/// uncompressed.
fn impl_deserialize_field(ty: &Type) -> (TokenStream, TokenStream) {
    // Check if type is a tuple.
    match ty {
        Type::Tuple(tuple) => {
            let mut compressed_fields = Vec::new();
            let mut uncompressed_fields = Vec::new();
            for elem_ty in tuple.elems.iter() {
                let (compressed, uncompressed) = impl_deserialize_field(elem_ty);
                compressed_fields.push(compressed);
                uncompressed_fields.push(uncompressed);
            }
            (
                quote! { (#(#compressed_fields)*), },
                quote! { (#(#uncompressed_fields)*), },
            )
        }
        _ => (
            quote! { CanonicalDeserialize::deserialize(reader)?, },
            quote! { CanonicalDeserialize::deserialize_uncompressed(reader)?, },
        ),
    }
}

fn impl_canonical_deserialize(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let deserialize_body;
    let deserialize_uncompressed_body;

    match ast.data {
        Data::Struct(ref data_struct) => {
            let mut tuple = false;
            let mut compressed_field_cases = Vec::<TokenStream>::new();
            let mut uncompressed_field_cases = Vec::<TokenStream>::new();
            for field in data_struct.fields.iter() {
                match &field.ident {
                    None => {
                        tuple = true;
                        let (compressed, uncompressed) = impl_deserialize_field(&field.ty);
                        compressed_field_cases.push(compressed);
                        uncompressed_field_cases.push(uncompressed);
                    }
                    // struct field without len_type
                    Some(ident) => {
                        let (compressed_field, uncompressed_field) = impl_deserialize_field(&field.ty);
                        compressed_field_cases.push(quote! { #ident: #compressed_field });
                        uncompressed_field_cases.push(quote! { #ident: #uncompressed_field });
                    }
                }
            }

            if tuple {
                deserialize_body = quote!({
                    Ok(#name (
                        #(#compressed_field_cases)*
                    ))
                });
                deserialize_uncompressed_body = quote!({
                    Ok(#name (
                        #(#uncompressed_field_cases)*
                    ))
                });
            } else {
                deserialize_body = quote!({
                    Ok(#name {
                        #(#compressed_field_cases)*
                    })
                });
                deserialize_uncompressed_body = quote!({
                    Ok(#name {
                        #(#uncompressed_field_cases)*
                    })
                });
            }
        }
        _ => panic!("Deserialize can only be derived for structs, {} is not a Struct", name),
    };

    let gen = quote! {
        impl #impl_generics CanonicalDeserialize for #name #ty_generics #where_clause {
            #[allow(unused_mut,unused_variables)]
            fn deserialize<R: Read>(reader: &mut R) -> Result<Self, SerializationError> {
                #deserialize_body
            }
            #[allow(unused_mut,unused_variables)]
            fn deserialize_uncompressed<R: Read>(reader: &mut R) -> Result<Self, SerializationError> {
                #deserialize_uncompressed_body
            }
        }
    };
    gen
}
