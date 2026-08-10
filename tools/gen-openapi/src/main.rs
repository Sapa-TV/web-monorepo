use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use backend::api::ApiDoc;
use utoipa::OpenApi;

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "generated/openapi.json".to_string());

    match ApiDoc::openapi().to_pretty_json() {
        Ok(json) => {
            let parent = Path::new(&path).parent();
            if let Some(parent) = parent {
                fs::create_dir_all(parent).ok();
            }
            match fs::write(&path, json) {
                Ok(()) => {
                    eprintln!("openapi spec written to {path}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("failed to write openapi spec: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("failed to serialize openapi spec: {e}");
            ExitCode::FAILURE
        }
    }
}
