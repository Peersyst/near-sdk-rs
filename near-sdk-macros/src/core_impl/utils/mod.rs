use proc_macro2::{Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::token::{And, Mut};
use syn::{GenericArgument, Path, PathArguments, Signature, Type};

/// Checks whether the given path is literally "Result".
/// Note that it won't match a fully qualified name `core::result::Result` or a type alias like
/// `type StringResult = Result<String, String>`.
pub(crate) fn path_is_result(path: &Path) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.iter().next().unwrap().ident == "Result"
}

/// Equivalent to `path_is_result` except that it works on `Type` values.
pub(crate) fn type_is_result(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => path_is_result(&type_path.path),
        _ => false,
    }
}

/// Extracts the Ok type from a `Result` type.
///
/// For example, given `Result<String, u8>` type it will return `String` type.
pub(crate) fn extract_ok_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_is_result(&type_path.path) => {
            // Get the first segment of the path (there should be only one, in fact: "Result"):
            let type_params = &type_path.path.segments.first()?.arguments;
            // We are interested in the first angle-bracketed param responsible for Ok type ("<String, _>"):
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => Some(params.args.first()?),
                _ => None,
            }?;
            // This argument must be a type:
            match generic_arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Checks whether the given path is literally "Vec".
/// Note that it won't match a fully qualified name `std::vec::Vec` or a type alias like
/// `type MyVec = Vec<String>`.
#[cfg(feature = "__abi-generate")]
fn path_is_vec(path: &Path) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.iter().next().unwrap().ident == "Vec"
}

/// Extracts the inner generic type from a `Vec<_>` type.
///
/// For example, given `Vec<String>` this function will return `String`.
#[cfg(feature = "__abi-generate")]
pub(crate) fn extract_vec_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_is_vec(&type_path.path) => {
            let type_params = &type_path.path.segments.first()?.arguments;
            let generic_arg = match type_params {
                // We are interested in the first (and only) angle-bracketed param:
                PathArguments::AngleBracketed(params) if params.args.len() == 1 => {
                    Some(params.args.first()?)
                }
                _ => None,
            }?;
            match generic_arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Extracts reference and mutability tokens from a `Type` object. Also, strips top-level lifetime binding if present.
pub(crate) fn extract_ref_mut(
    ty: &Type,
    span: Span,
) -> syn::Result<(Option<And>, Option<Mut>, Type)> {
    match ty {
        x @ (Type::Array(_) | Type::Path(_) | Type::Tuple(_) | Type::Group(_)) => {
            Ok((None, None, (*x).clone()))
        }
        Type::Reference(r) => Ok((Some(r.and_token), r.mutability, (*r.elem.as_ref()).clone())),
        _ => Err(syn::Error::new(span, "Unsupported contract API type.")),
    }
}

/// Checks that the method signature is supported in the NEAR Contract API.
pub(crate) fn sig_is_supported(sig: &Signature) -> syn::Result<()> {
    if sig.asyncness.is_some() {
        return Err(syn::Error::new(sig.span(), "Contract API is not allowed to be async."));
    }
    if sig.abi.is_some() {
        return Err(syn::Error::new(
            sig.span(),
            "Contract API is not allowed to have binary interface.",
        ));
    }
    if sig.variadic.is_some() {
        return Err(syn::Error::new(
            sig.span(),
            "Contract API is not allowed to have variadic arguments.",
        ));
    }

    Ok(())
}

fn _sanitize_self(typ: TokenStream2, replace_with: &TokenStream2) -> TokenStream2 {
    let trees = typ.into_iter().map(|t| match t {
        TokenTree::Ident(ident) if ident == "Self" => replace_with
            .clone()
            .into_iter()
            .map(|mut t| {
                t.set_span(ident.span());
                t
            })
            .collect::<TokenStream2>(),
        TokenTree::Group(group) => {
            let stream = _sanitize_self(group.stream(), replace_with);
            TokenTree::Group(Group::new(group.delimiter(), stream)).into()
        }
        rest => rest.into(),
    });
    trees.collect()
}

pub fn sanitize_self(typ: &Type, replace_with: &TokenStream2) -> syn::Result<Type> {
    syn::parse2(_sanitize_self(quote! { #typ }, replace_with)).map_err(|original| {
        syn::Error::new(original.span(), "Self sanitization failed. Please report this as a bug.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_self_works() {
        let typ: Type = syn::parse_str("Self").unwrap();
        let replace_with: TokenStream2 = syn::parse_str("MyType").unwrap();
        let sanitized = sanitize_self(&typ, &replace_with).unwrap();
        assert_eq!(quote! { #sanitized }.to_string(), "MyType");

        let typ: Type = syn::parse_str("Vec<Self>").unwrap();
        let replace_with: TokenStream2 = syn::parse_str("MyType").unwrap();
        let sanitized = sanitize_self(&typ, &replace_with).unwrap();
        assert_eq!(quote! { #sanitized }.to_string(), "Vec < MyType >");

        let typ: Type = syn::parse_str("Vec<Vec<Self>>").unwrap();
        let replace_with: TokenStream2 = syn::parse_str("MyType").unwrap();
        let sanitized = sanitize_self(&typ, &replace_with).unwrap();
        assert_eq!(quote! { #sanitized }.to_string(), "Vec < Vec < MyType > >");

        let typ: Type = syn::parse_str("Option<[(Self, Result<Self, ()>); 2]>").unwrap();
        let replace_with: TokenStream2 = syn::parse_str("MyType").unwrap();
        let sanitized = sanitize_self(&typ, &replace_with).unwrap();
        assert_eq!(
            quote! { #sanitized }.to_string(),
            "Option < [(MyType , Result < MyType , () >) ; 2] >"
        );
    }
}
