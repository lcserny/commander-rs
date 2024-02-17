use std::{fs, io::{self, ErrorKind, Error}, env};

use commander::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.is_empty() || args.len() != 2 {
        return Err(Error::new(ErrorKind::InvalidInput, "please provide file name for spec generation"));
    }
    fs::write(&args[1], ApiDoc::openapi().to_yaml().unwrap())
}