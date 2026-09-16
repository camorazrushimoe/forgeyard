use std::process::ExitCode;

use forgeyard_core::{factory_root, load_tokens, resolve_mcp_port, serve_mcp, Exit};

fn main() -> ExitCode {
    let root = factory_root();
    let tokens = load_tokens(&root).unwrap_or_default();
    let token = tokens.get("mcp.bind_token").unwrap_or("");
    if token.is_empty() {
        eprintln!("mcp: refused");
        return Exit::Tokens.into();
    }
    let port = resolve_mcp_port(&root);
    match serve_mcp(&root, port, token) {
        Ok(()) => Exit::Ok.into(),
        Err(e) => {
            eprintln!("{e}");
            e.exit().into()
        }
    }
}
