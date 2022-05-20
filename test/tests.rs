use baker::Bake;

#[test]
fn test() {
  #[derive(Bake, Clone)]
  #[baked(name = "Baked")]
  struct Test {
    #[baked(ignore)]
    pub _ignored: (),
    #[baked(type = "String", map_fn(bake = "|u| u.new_type.first().unwrap().to_owned()", view = "|u| u.new_type.first().unwrap()"))]
    pub new_type: Vec<String>,
    #[baked(name = "name", map = "self.field")]
    pub field: String,
  }

  let test = Test {
    _ignored: (),
    new_type: vec!["1".to_string(), "2".to_string()],
    field: "test".to_string(),
  };

  let baked = test.clone().bake();

  assert_eq!(test.new_type.first().unwrap().to_owned(), baked.new_type);
  assert_eq!(test.field, baked.name);

  let viewed = test.view();

  assert_eq!(test.new_type.first().unwrap(), viewed.new_type);
  assert_eq!(&test.field, viewed.name);
}

#[test]
fn fail() {
  #[derive(Bake, Clone)]
  #[baked(name = "Baked", error_type = "String")]
  struct Test {
    #[baked(type = "String", map_fn(try_bake = "|u| Ok::<String, String>(u._new_type.first().unwrap().to_owned())", try_view = "|u| Err(\"failed\".to_string())"))]
    pub _new_type: Vec<String>,
    pub _field: String,
  }

  let test = Test {
    _new_type: vec!["1".to_string(), "2".to_string()],
    _field: "test".to_string(),
  };

  if let Err(err) = test.clone().bake() {
    assert_eq!(err, "failed".to_string());
  } else {
    panic!("bake() should have failed");
  }

  if let Err(err) = test.view() {
    assert_eq!(err, "failed".to_string());
  } else {
    panic!("view() should have failed");
  }
}