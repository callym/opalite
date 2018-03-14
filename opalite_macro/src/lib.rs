#![recursion_limit="128"]

#![feature(
    conservative_impl_trait,
    crate_in_paths,
    match_default_bindings,
    nll,
    proc_macro,
    universal_impl_trait,
)]
extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use] extern crate quote;

use proc_macro::TokenStream;
use syn::{ Attribute, DeriveInput, Data, Fields, Ident, Lit, Meta };
use quote::Tokens;

fn attr_to_number<'a>(attrs: impl Iterator<Item = &'a Attribute>, attr_name: &str, default: Option<u64>) -> u64 {
    let default = default.expect(&format!("{} is required.", attr_name));

    let attr = attrs
        .filter(|a| {
            let path = a.path.clone();
            quote!(#path) == quote!(#attr_name)
        })
        .map(|a| a.interpret_meta())
        .filter(|a| a.is_some())
        .map(|a| a.unwrap())
        .next();

    match attr {
        Some(attr) => {
            match attr {
                Meta::NameValue(meta) => match meta.lit {
                    Lit::Int(lit_int) => lit_int.value(),
                    _ => default,
                },
                _ => default,
            }
        },
        None => default,
    }
}

fn field_to_format(field: Tokens) -> Tokens {
    let pre = quote!(hal::format::Format);

    if field == quote!(f32) {
        return quote!(#pre::R32Float);
    }

    if field == quote!([f32; 2]) {
        return quote!(#pre::Rg32Float);
    }

    if field == quote!([f32; 3]) {
        return quote!(#pre::Rgb32Float);
    }

    if field == quote!([f32; 4]) {
        return quote!(#pre::Rgba32Float);
    }

    quote!(UndefinedType)
}

fn attribute_desc(location: u64, binding: u64, format: Tokens) -> Tokens {
    let location = location as u32;
    let binding = binding as u32;

    quote! {
        hal::pso::AttributeDesc {
            location: #location,
            binding: #binding,
            element: hal::pso::Element {
                format: #format,
                offset: offset,
            },
        },
    }
}

#[proc_macro_derive(BufferData, attributes(location, binding))]
pub fn derive_buffer_data(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let location = attr_to_number(input.attrs.iter(), "location", Some(0));
    let binding = attr_to_number(input.attrs.iter(), "binding", Some(0));

    let types: Vec<syn::Type> = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => {
                fields.named.iter().map(|f| f.ty.clone()).collect()
            },
            Fields::Unnamed(fields) => {
                fields.unnamed.iter().map(|f| f.ty.clone()).collect()
            }
            _ => vec![],
        },
        _ => vec![],
    };
    let fields = types.iter()
        .map(|f| field_to_format(quote!(#f)))
        .map(|f| attribute_desc(location, binding, f))
        .collect::<Vec<_>>();

    let dummy = Ident::from(format!("opalite___derive_buffer_data___{}", name));

    let imports = if cfg!(feature = "internal") {
        quote! {
            extern crate std;
            use ::{ hal, renderer };
            use renderer::BufferData;
        }
    } else {
        quote! {
            extern crate std;
            extern crate opalite;
            use self::opalite::{ hal, renderer };
        }
    };

    let derive = quote! {
        mod #dummy {
            #imports

            impl #impl_generics BufferData for #name #ty_generics #where_clause {
                fn desc() -> Vec<hal::pso::AttributeDesc> {
                    let mut attrs = vec![];
                    let mut offset = 0;

                    #(
                        attrs.push(#fields);
                        offset += std::mem::size_of::<#types>() as u32;
                    )*

                    attrs
                }
            }
        }
    };

    let derive: String = format!("{}", derive);
    let derive = syn::parse_str::<syn::ItemMod>(&derive).unwrap();
    quote!(#derive).into()
}
