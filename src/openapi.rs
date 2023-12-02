use std::fs;

use serde::Deserialize;
use utoipa::{ToSchema, OpenApi};

// TODO

#[derive(Deserialize, ToSchema)]
struct Person {
    id: i64,
    name: String,
}

#[derive(OpenApi)]
#[openapi(components(schemas(Person)))]
pub struct ApiDoc;

fn main() {
    let doc = gen_my_openapi();
    fs::write("./spec.yml", doc);
}

fn gen_my_openapi() -> String {
  ApiDoc::openapi().to_yaml().unwrap()
}