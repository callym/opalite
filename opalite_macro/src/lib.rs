#![recursion_limit="128"]

#![feature(
    crate_in_paths,
    match_default_bindings,
    nll,
    proc_macro,
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

    if field == quote!(f32) || field == quote!([f32; 1]) {
        return quote!(#pre::R32Float);
    }

    if field == quote!([f32; 2]) || field == quote!(Vector2<f32>) {
        return quote!(#pre::Rg32Float);
    }

    if field == quote!([f32; 3]) || field == quote!(Vector3<f32>) {
        return quote!(#pre::Rgb32Float);
    }

    if field == quote!([f32; 4]) || field == quote!(Vector4<f32>) {
        return quote!(#pre::Rgba32Float);
    }

    if field == quote!(u32) || field == quote!([u32; 1]) {
        return quote!(#pre::R32Uint);
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

#[proc_macro_derive(BufferData, attributes(binding, uniform))]
pub fn derive_buffer_data(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let binding = attr_to_number(input.attrs.iter(), "binding", Some(0));

    let uniform = input.attrs.iter()
        .filter(|a| {
            let path = a.path.clone();
            quote!(#path) == quote!(uniform)
        })
        .map(|a| a.interpret_meta())
        .filter(|a| a.is_some())
        .map(|a| a.unwrap())
        .next()
        .is_some();

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
        .enumerate()
        .map(|(i, f)| attribute_desc(i as u64, binding, f))
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

    let desc = if uniform {
        quote! {
            vec![]
        }
    } else {
        quote! {
            let mut attrs = vec![];
            let mut offset = 0;

            #(
                attrs.push(#fields);
                offset += std::mem::size_of::<#types>() as u32;
            )*

            attrs
        }
    };

    let derive = quote! {
        #[allow(non_snake_case)]
        mod #dummy {
            #![allow(unused_assignments, unused_imports, dead_code)]

            #imports

            impl #impl_generics BufferData for #name #ty_generics #where_clause {
                const STRIDE: u64 = std::mem::size_of::<Self>() as u64;

                fn desc() -> Vec<hal::pso::AttributeDesc> {
                    #desc
                }
            }
        }
    };

    let derive: String = format!("{}", derive);
    let derive = syn::parse_str::<syn::ItemMod>(&derive).unwrap();
    quote!(#derive).into()
}

#[proc_macro_derive(PushConstant)]
pub fn derive_push_constant(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let dummy = Ident::from(format!("opalite___derive_push_constant___{}", name));

    let imports = if cfg!(feature = "internal") {
        quote! {
            extern crate std;
            use ::{ bincode, hal, renderer };
            use renderer::PushConstant;
            use bincode::serialize;
        }
    } else {
        quote! {
            extern crate std;
            extern crate opalite;
            use self::opalite::{ bincode::serialize, hal, renderer };
        }
    };


    let derive = quote! {
        #[allow(non_snake_case)]
        mod #dummy {
            #![allow(unused_assignments, unused_imports, dead_code)]

            #imports

            impl #impl_generics PushConstant for #name #ty_generics #where_clause {
                const SIZE: u32 = (std::mem::size_of::<Self>() / 4) as u32;

                fn data(&self) -> Vec<u32> {
                    let data = serialize(&self).unwrap();
                    unsafe { mem::transmute::<Vec<u8>, Vec<u32>>(data) }
                }
            }
        }
    };

    let derive: String = format!("{}", derive);
    let derive = syn::parse_str::<syn::ItemMod>(&derive).unwrap();
    quote!(#derive).into()
}
