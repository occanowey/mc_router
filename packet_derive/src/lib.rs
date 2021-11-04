use proc_macro2::Literal;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Packet, attributes(id))]
pub fn packet(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let packet_id: Literal = input.attrs.get(0)
        .and_then(|attr| attr.parse_args().ok())
        .expect("Expected packet id");

    proc_macro::TokenStream::from(quote! {
        impl Packet for #name {
            const PACKET_ID: i32 = #packet_id;
        }
    })
}