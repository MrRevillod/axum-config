use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Expr, Lit, Meta, parse_macro_input};

#[proc_macro_attribute]
pub fn config(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match config_impl(args, &input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn config_impl(args: TokenStream, input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let meta = syn::parse::<Meta>(args)?;

    let nv = meta.require_name_value().map_err(|_| {
        Error::new_spanned(&meta, r#"expected format: #[config(key = "section_name")]"#)
    })?;

    if !nv.path.is_ident("key") {
        return Err(Error::new_spanned(&nv.path, "expected `key` attribute"));
    }

    let Expr::Lit(expr_lit) = &nv.value else {
        return Err(Error::new_spanned(
            &nv.value,
            "expected string literal for key",
        ));
    };

    let Lit::Str(lit_str) = &expr_lit.lit else {
        return Err(Error::new_spanned(
            &expr_lit.lit,
            "expected string literal for key",
        ));
    };

    let expanded = quote! {
        #input

        impl ::thisconfig::ConfigItem for #name {
            fn key() -> &'static str {
                #lit_str
            }
        }
    };

    Ok(expanded.into())
}
