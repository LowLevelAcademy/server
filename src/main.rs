//! This is a simple server that compiles snippets into WebAssembly code.
//! It is heavily based on the [rust-playground code][rust-playground]
//!
//! [rust-playground]: (https://github.com/integer32llc/rust-playground/tree/master/ui

mod sandbox;

use rocket::{
    config::Environment,
    http::ContentType,
    post,
    response::{content, status},
};
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use sandbox::Sandbox;
use serde::Deserialize;
use std::{env, fs::File, io::prelude::*};

#[derive(Deserialize, Debug)]
struct ExecuteCommand {
    code: String,
}

#[post("/", data = "<req>")]
async fn execute(
    req: Json<ExecuteCommand>,
) -> Result<content::Content<File>, status::BadRequest<String>> {
    let mut source_path = env::temp_dir();
    source_path.push("req.rs");

    {
        let mut source_file = File::create(source_path.clone()).expect("could not create file");
        source_file
            .write_all(req.code.as_bytes())
            .expect("could not write source text");
    }

    let output = Sandbox::new()
        .map_err(|e| status::BadRequest(Some(e.to_string())))? // todo internal server error
        .compile(&req.code)
        .await
        .map_err(|e| status::BadRequest(Some(e.to_string())))?;

    if !output.success {
        return Err(status::BadRequest(Some(output.stderr)));
    }

    Ok(content::Content(ContentType::WASM, output.wasm.unwrap()))
}

#[rocket::launch]
fn rocket() -> rocket::Rocket {
    let mut rocket = rocket::ignite();

    if Environment::active().unwrap().is_dev() {
        // Serve static files
        let static_files_path = env::var("STATIC_FILES").unwrap();
        rocket = rocket.mount("/", StaticFiles::from(static_files_path));
    }

    rocket.mount("/compile", rocket::routes![execute])
}
