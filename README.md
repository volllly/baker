# Baker

[![crates.io](https://img.shields.io/crates/v/baker)](https://crates.io/crates/baker)
[![docs.rs](https://docs.rs/baker/badge.svg)](https://docs.rs/baker/)

Baker provides a procedural macro for creating a final (baked) struct from an intermediate struct.

Lets say you have a struct that gets parsed from your args or a config file and you need to process this data into similar struct before using it.

```rust
struct Cli {
  pub urls: Vec<String>,
  pub add_slash_to_end: bool
}

struct CliBaked {
  pub urls: Vec<String>,
}

impl Cli {
  pub fn bake(self) -> CliBaked {
    CliBaked {
      urls: self.urls.into_iter().map(|u| if self.add_slash_to_end && !u.ends_with('/') { u + "/" } else { u }).collect::<Vec<_>>()
    }
  }
}
```

The same thing can be achieved using Baker.

```rust
use baker::Bake;

#[derive(Bake)]
#[baked(name = "CliBaked")]
struct Cli {
  #[baked(map_fn(bake = "|cli| cli.urls.iter().map(|u| if cli.add_slash_to_end && !u.ends_with('/') { u.to_string() + \"/\" } else { u.to_string() }).collect::<Vec<_>>()"))]
  pub urls: Vec<String>,
  #[baked(ignore)]
  pub add_slash_to_end: bool,
}
```