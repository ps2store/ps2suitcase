use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Serialize, attributes(serialize))]
pub fn serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident.clone();

    let serialize_impl = match input.data.clone() {
        Data::Struct(data_struct) => {
            let field_serializations = data_struct.fields.iter().map(|field| {
                let attrs: Vec<_> = field
                    .attrs
                    .iter()
                    .map(|attr| {
                        let name = attr.path().get_ident().unwrap().to_string();

                        quote! {
                            println!("{}", #name);
                        }
                    })
                    .collect();

                quote! {
                    #(#attrs)*
                }
            });

            quote! {
                impl #struct_name {
                    pub fn serialize(&self) -> String {
                        let mut output = String::new();
                        #(#field_serializations)*
                        output
                    }
                }
            }
        }
        _ => {
            quote! {
                compile_error!("Serialize can only be derived for structs");
            }
        }
    };

    let output = quote! {
        #serialize_impl
    };

    TokenStream::from(output)
}
