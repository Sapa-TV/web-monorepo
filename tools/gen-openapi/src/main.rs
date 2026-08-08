use std::process::ExitCode;

use utoipa::OpenApi;

fn main() -> ExitCode {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "generated/openapi.json".to_string());

    match backend::api::ApiDoc::openapi().to_pretty_json() {
        Ok(json) => {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("failed to write openapi spec: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("openapi spec written to {path}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("failed to serialize openapi spec: {e}");
            ExitCode::FAILURE
        }
    }
}