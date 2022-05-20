use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use somok::Somok;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(baked), supports(struct_named))]
pub struct Struct {
  pub ident: syn::Ident,
  pub vis: syn::Visibility,
  pub generics: syn::Generics,
  #[darling()]
  pub data: darling::ast::Data<(), Field>,

  pub name: syn::Ident,

  #[darling(default)]
  pub name_view: Option<syn::Ident>,

  #[darling(default)]
  pub error_type: Option<syn::Path>,
}

#[derive(Debug, FromField)]
#[darling(attributes(baked), and_then = "Self::valid_combination")]
pub struct Field {
  ident: Option<syn::Ident>,
  vis: syn::Visibility,
  ty: syn::Type,

  #[darling(default, and_then = "Self::is_path")]
  map: Option<darling::util::SpannedValue<syn::Expr>>,

  #[darling(default)]
  map_fn: Option<darling::util::SpannedValue<MapFn>>,

  #[darling(default)]
  ignore: darling::util::Flag,

  #[darling(default, rename = "type")]
  new_type: Option<darling::util::SpannedValue<syn::Type>>,

  #[darling(default)]
  name: Option<darling::util::SpannedValue<syn::Ident>>,
}

impl Field {
  fn is_path(value: Option<darling::util::SpannedValue<syn::Expr>>) -> darling::Result<Option<darling::util::SpannedValue<syn::Expr>>> {
    if let Some(value) = &value {
      if !matches!(value.as_ref(), syn::Expr::Field(_)) {
        darling::Error::custom("Expression must be a field").with_span(&value.span()).error()?
      }
    };

    value.okay()
  }

  fn valid_combination(self) -> darling::Result<Self> {
    if self.ignore.is_some() {
      if let Some(map) = &self.map {
        darling::Error::custom("Attribute map cannot be set when field is ignored").with_span(&map.span()).error()?;
      }

      if let Some(new_type) = &self.new_type {
        darling::Error::custom("Attribute type cannot be set when field is ignored").with_span(&new_type.span()).error()?;
      }

      if let Some(name) = &self.name {
        darling::Error::custom("Attribute name cannot be set when field is ignored").with_span(&name.span()).error()?;
      }
    } else {
      if self.map.is_some() && self.map_fn.is_some() {
        darling::Error::custom("map_fn must not be set if map is set")
          .with_span(&self.map_fn.as_ref().unwrap().span())
          .error()?;
      }

      if self.map.is_none() && self.map_fn.is_none() {
        darling::Error::custom("Either or map_fn must be set").error()?;
      }
    }

    self.okay()
  }
}

#[derive(Debug, FromMeta)]
#[darling(and_then = "Self::valid_combination")]
pub struct MapFn {
  #[darling(default, and_then = "Self::is_closure")]
  view: Option<darling::util::SpannedValue<syn::Expr>>,

  #[darling(default, and_then = "Self::is_closure")]
  bake: Option<darling::util::SpannedValue<syn::Expr>>,

  #[darling(default, and_then = "Self::is_closure")]
  try_view: Option<darling::util::SpannedValue<syn::Expr>>,

  #[darling(default, and_then = "Self::is_closure")]
  try_bake: Option<darling::util::SpannedValue<syn::Expr>>,
}

impl MapFn {
  fn is_closure(value: Option<darling::util::SpannedValue<syn::Expr>>) -> darling::Result<Option<darling::util::SpannedValue<syn::Expr>>> {
    if let Some(value) = &value {
      if !matches!(value.as_ref(), syn::Expr::Closure(_)) {
        darling::Error::custom("Expression must be a closure.").with_span(&value.span()).error()?
      }
    };

    value.okay()
  }

  fn valid_combination(self) -> darling::Result<Self> {
    if let Some(try_bake) = &self.try_bake {
      if self.bake.is_some() {
        darling::Error::custom("try_bake cannot be set if bake is set").with_span(&try_bake.span()).error()?
      }
    };

    if let Some(try_view) = &self.try_view {
      if self.view.is_some() {
        darling::Error::custom("try_view cannot be set if view is set").with_span(&try_view.span()).error()?
      }
    };

    self.okay()
  }
}

pub fn bake(
  Struct {
    ident,
    vis,
    generics,
    data,
    name,
    name_view,
    error_type,
  }: Struct,
) -> Result<TokenStream, TokenStream> {
  let name_view = name_view.unwrap_or_else(|| format_ident!("{}View", &name));
  let mut generics_view = generics.clone();
  generics_view.params.insert(0, syn::parse_quote! { '__a });

  let fields = data.take_struct().unwrap().fields.into_iter().filter(|f| f.ignore.is_none()).collect::<Vec<_>>();

  let baked_fields = fields.iter().map(
    |Field {
       ident,
       vis,
       ty,
       new_type,
       name,
       map: _,
       ignore: _,
       map_fn: _,
     }| {
      let name = name.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ident.as_ref().unwrap());
      let ty = new_type.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ty);
      quote! { #vis #name: #ty }
    },
  );

  let viewed_fields = fields.iter().map(
    |Field {
       ident,
       vis,
       ty,
       new_type,
       name,
       map: _,
       ignore: _,
       map_fn: _,
     }| {
      let name = name.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ident.as_ref().unwrap());
      let ty = new_type.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ty);
      quote! { #vis #name: &'__a #ty }
    },
  );

  let has_map_fn = fields.iter().filter_map(|f| f.map_fn.as_ref()).fold((false, false), |mut acc, m| {
    acc.0 |= m.view.is_some() || m.try_view.is_some();
    acc.1 |= m.bake.is_some() || m.try_bake.is_some();

    acc
  });

  let errors = fields
    .iter()
    .filter_map(|f| {
      let mut errors: Vec<darling::Error> = vec![];

      if f.map.is_some() {
        return None;
      }

      let map_fn = f.map_fn.as_ref().unwrap();

      if has_map_fn.0 && (map_fn.view.is_none() && map_fn.try_view.is_none()) {
        errors.push(darling::Error::custom("map_fn(view) or map_fn(try_view) is missing").with_span(&map_fn.span()));
      }

      if has_map_fn.1 && (map_fn.bake.is_none() && map_fn.try_bake.is_none()) {
        errors.push(darling::Error::custom("map_fn(bake) or map_fn(try_bake) is missing").with_span(&map_fn.span()));
      }

      if errors.is_empty() {
        None
      } else {
        errors.some()
      }
    })
    .flatten()
    .collect::<Vec<_>>();

  if !errors.is_empty() {
    return darling::Error::multiple(errors).write_errors().error();
  }

  let mut try_view = false;

  let (view, errors): (Vec<_>, Vec<_>) = fields
    .iter()
    .filter_map(
      |Field {
         ident,
         vis: _,
         ty: _,
         map,
         map_fn,
         ignore: _,
         new_type: _,
         name,
       }| {
        let name = name.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ident.as_ref().unwrap());

        let value = if let Some(map) = map {
          let map = &**map;
          quote! { &#map }
        } else if let Some(map_fn) = map_fn {
          let (view_spanned, try_variant) = if let Some(view_spanned) = &map_fn.view {
            (view_spanned, false)
          } else if let Some(view_spanned) = &map_fn.try_view {
            try_view = true;
            (view_spanned, true)
          } else {
            return None;
          };

          let mut view = if let syn::Expr::Closure(view) = view_spanned.as_ref().clone() { view } else { panic!() };
          let self_param = view
            .inputs
            .first_mut()
            .ok_or_else(|| darling::Error::custom("Closure needs a parameter").with_span(&view_spanned.span()));

          let mut self_param = match self_param {
            Ok(self_param) => self_param,
            Err(err) => return err.error().some(),
          };

          let ident = if let syn::Pat::Ident(self_param_ident) = &mut self_param {
            &self_param_ident.ident
          } else {
            return darling::Error::custom("Closure parameter needs to be an identifier").with_span(&view_spanned.span()).error().some();
          };
          *self_param = syn::Pat::Type(syn::PatType {
            attrs: vec![],
            pat: syn::parse_quote! { #ident },
            colon_token: syn::parse_quote! { : },
            ty: syn::parse_quote! { &'__a Self },
          });
          if try_variant {
            quote! { (#view)(self)? }
          } else {
            quote! { (#view)(self) }
          }
        } else {
          return None;
        };
        quote! { #name: #value }.okay().some()
      },
    )
    .partition(|f| f.is_ok());

  if !errors.is_empty() {
    return darling::Error::multiple(errors.into_iter().filter_map(|e| e.err()).collect::<Vec<_>>()).write_errors().error();
  }

  let view = view.into_iter().filter_map(|f| f.ok()).collect::<Vec<_>>();

  let mut try_bake = false;

  let (bake, errors): (Vec<_>, Vec<_>) = fields
    .iter()
    .filter_map(
      |Field {
         ident,
         vis: _,
         ty: _,
         map,
         map_fn,
         ignore: _,
         new_type: _,
         name,
       }| {
        let name = name.as_ref().map(|n| n.as_ref()).unwrap_or_else(|| ident.as_ref().unwrap());

        let value = if let Some(map) = map {
          let map = &**map;
          quote! { #map }
        } else if let Some(map_fn) = map_fn {
          let (bake_spanned, try_variant) = if let Some(bake_spanned) = &map_fn.bake {
            (bake_spanned, false)
          } else if let Some(bake_spanned) = &map_fn.try_bake {
            try_bake = true;
            (bake_spanned, true)
          } else {
            return None;
          };

          let mut bake = if let syn::Expr::Closure(bake) = bake_spanned.as_ref().clone() { bake } else { panic!() };
          let self_param = bake
            .inputs
            .first_mut()
            .ok_or_else(|| darling::Error::custom("Closure needs a parameter").with_span(&bake_spanned.span()));

          let mut self_param = match self_param {
            Ok(self_param) => self_param,
            Err(err) => return err.error().some(),
          };

          let ident = if let syn::Pat::Ident(self_param_ident) = &mut self_param {
            &self_param_ident.ident
          } else {
            return darling::Error::custom("Closure parameter needs to be an identifier").with_span(&bake_spanned.span()).error().some();
          };
          *self_param = syn::Pat::Type(syn::PatType {
            attrs: vec![],
            pat: syn::parse_quote! { #ident },
            colon_token: syn::parse_quote! { : },
            ty: syn::parse_quote! { &Self },
          });
          if try_variant {
            quote! { (#bake)(&self)? }
          } else {
            quote! { (#bake)(&self) }
          }
        } else {
          return None;
        };
        quote! { #name: #value }.okay().some()
      },
    )
    .partition(|f| f.is_ok());

  if !errors.is_empty() {
    return darling::Error::multiple(errors.into_iter().filter_map(|e| e.err()).collect::<Vec<_>>()).write_errors().error();
  }

  let bake = bake.into_iter().filter_map(|f| f.ok()).collect::<Vec<_>>();

  let view = if view.len() == fields.len() {
    let return_type = if try_view {
      quote! { core::result::Result<#name_view #generics_view, #error_type> }
    } else {
      quote! { #name_view #generics_view }
    };

    let result = if try_bake {
      quote! {
        Ok(#name_view {
          #( #view ),*
        })
      }
    } else {
      quote! {
        #name_view {
          #( #view ),*
        }
      }
    };

    quote! {
      pub fn view<'__a>(&'__a self) -> #return_type {
        #result
      }
    }
  } else {
    quote! {}
  };

  let bake = if bake.len() == fields.len() {
    let return_type = if try_bake {
      quote! { core::result::Result<#name #generics, #error_type> }
    } else {
      quote! { #name #generics }
    };

    let result = if try_bake {
      quote! {
        Ok(#name {
          #( #bake ),*
        })
      }
    } else {
      quote! {
        #name {
          #( #bake ),*
        }
      }
    };

    quote! {
      pub fn bake(self) -> #return_type {
        #result
      }
    }
  } else {
    quote! {}
  };

  quote! {
    #vis struct #name #generics {
      #( #baked_fields ),*
    }

    #vis struct #name_view #generics_view {
      #( #viewed_fields ),*
    }

    impl #generics #ident #generics {
      #view

      #bake
    }
  }
  .okay()
}
