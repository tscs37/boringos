#![no_std]
#![feature(type_alias_enum_variants)]

extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro;
use proc_macro::*;
use quote::quote;
use syn::{parse_macro_input, Expr, Ident, Token, Type, Visibility};
use syn::{TypeBareFn, spanned::Spanned, BareFnArg};
use syn::parse::{Parse, ParseStream, Result};

struct ImportFunction {
  visibility: Visibility,
  name: Ident,
  asname: Ident,
  ty: TypeBareFn,
}

fn transform_bareargs(ty: BareFnArg) -> BareFnArg {
  let ty = ty.clone();
  let mut pnum = 0;
  for x in ty.into_iter() {
    x.name = Some(syn::BareFnArgName::Named(
      Ident::new(format!("arg{}", pnum++), x.span())
    ))
    p++;
  }
  ty
}

impl Parse for ImportFunction {
  fn parse(input: ParseStream) -> Result<Self> {
    let visibility: Visibility = input.parse()?;
    let name: Ident = input.parse()?;
    let asname: Ident = if let Ok(_) = input.parse::<Token![as]>() {
      input.parse()?
    } else { name.clone() };
    let ty: Type = input.parse()?;
    let ty: TypeBareFn = if let Type::BareFn(ty) = ty {
      ty
    } else {
      return Err(syn::Error::new(ty.span(), "must be a bare function type"));
    };
    if ty.lifetimes.is_some() {
      return Err(syn::Error::new(ty.span(), "Function must not have a lifetime"));
    }
    if ty.unsafety.is_some() {
      return Err(syn::Error::new(ty.span(), "Function must not be unsafe"));
    }
    if ty.abi.is_some() {
      //TODO: implement ABI
      return Err(syn::Error::new(ty.span(), "Function must not have ABI Option"));
    }

    Ok(ImportFunction{
      visibility,
      name,
      asname,
      ty,
    })
  }
}

#[proc_macro]
pub fn import_function(input: TokenStream) -> TokenStream {
  let ImportFunction {
    visibility,
    name,
    asname,
    ty,
  } = parse_macro_input!(input as ImportFunction);


  TokenStream::new()
}