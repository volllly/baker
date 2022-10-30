#![doc = include_str!("../README.md")]

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod bake;

#[proc_macro_derive(Bake, attributes(baked))]
pub fn restcrab(input: TokenStream) -> TokenStream {
  let input_parsed = match bake::Struct::from_derive_input(&parse_macro_input!(input as DeriveInput)) {
    Ok(v) => v,
    Err(e) => {
      return TokenStream::from(e.write_errors());
    }
  };

  match bake::bake(input_parsed) {
    Ok(ok) => ok,
    Err(err) => err,
  }
  .into()
}
