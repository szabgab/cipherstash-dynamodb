use crate::settings::Settings;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

pub(crate) fn derive_decryptable(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let settings = Settings::builder(&input)
        .container_attributes(&input)?
        .field_attributes(&input)?
        .build()?;

    let protected_attributes = settings.protected_attributes();
    let plaintext_attributes = settings.plaintext_attributes();

    let protected_attributes_cow = settings.protected_attributes()
        .into_iter()
        .map(|x| quote!{ std::borrow::Cow::Borrowed(#x) });

    let plaintext_attributes_cow = settings.plaintext_attributes()
        .into_iter()
        .map(|x| quote!{ std::borrow::Cow::Borrowed(#x) });

    let skipped_attributes = settings.skipped_attributes();
    let ident = settings.ident();

    let type_name = &settings.type_name;

    let sort_key_prefix_impl = if let Some(prefix) = &settings.sort_key_prefix {
        quote! { Some(std::borrow::Cow::Borrowed(#prefix)) }
    } else {
        quote! { None }
    };

    let from_unsealed_impl = protected_attributes
        .iter()
        .map(|attr| {
            let attr_ident = format_ident!("{attr}");

            quote! {
                #attr_ident: ::cipherstash_dynamodb::traits::TryFromPlaintext::try_from_plaintext(unsealed.get_protected(#attr)?.to_owned())?
            }
        })
        .chain(plaintext_attributes.iter().map(|attr| {
            let attr_ident = format_ident!("{attr}");

            quote! {
                #attr_ident: ::cipherstash_dynamodb::traits::TryFromTableAttr::try_from_table_attr(unsealed.get_plaintext(#attr)?)?
            }
        }))
        .chain(skipped_attributes.iter().map(|attr| {
            let attr_ident = format_ident!("{attr}");

            quote! {
                #attr_ident: Default::default()
            }
        }));

    let expanded = quote! {
        #[automatically_derived]
        impl cipherstash_dynamodb::traits::Decryptable for #ident {
            #[inline]
            fn type_name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(#type_name)
            }

            #[inline]
            fn sort_key_prefix() -> Option<std::borrow::Cow<'static, str>> {
                #sort_key_prefix_impl
            }

            fn protected_attributes() -> std::borrow::Cow<'static, [std::borrow::Cow<'static, str>]> {
                std::borrow::Cow::Borrowed(&[#(#protected_attributes_cow,)*])
            }

            fn plaintext_attributes() -> std::borrow::Cow<'static, [std::borrow::Cow<'static, str>]> {
                std::borrow::Cow::Borrowed(&[#(#plaintext_attributes_cow,)*])
            }

            fn from_unsealed(unsealed: cipherstash_dynamodb::crypto::Unsealed) -> Result<Self, cipherstash_dynamodb::crypto::SealError> {
                Ok(Self {
                    #(#from_unsealed_impl,)*
                })
            }
        }
    };

    Ok(expanded)
}
