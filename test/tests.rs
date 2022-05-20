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
