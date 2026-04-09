//! JSON interchange for LIR AST (Phase 2): versioned envelope + `serde_json`.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ast::Program;

/// `schema_version` in [`LirAstDocument`]; bump when the JSON shape changes.
pub const AST_JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LirAstDocument {
    pub schema_version: u32,
    pub program: Program,
}

#[derive(Debug)]
pub enum AstJsonError {
    Json(serde_json::Error),
    UnsupportedSchemaVersion { got: u32 },
}

impl fmt::Display for AstJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AstJsonError::Json(e) => write!(f, "{e}"),
            AstJsonError::UnsupportedSchemaVersion { got } => {
                write!(
                    f,
                    "unsupported schema_version {got} (expected {AST_JSON_SCHEMA_VERSION})"
                )
            }
        }
    }
}

impl std::error::Error for AstJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AstJsonError::Json(e) => Some(e),
            AstJsonError::UnsupportedSchemaVersion { .. } => None,
        }
    }
}

/// Serialize a program as pretty-printed JSON with a versioned envelope.
pub fn serialize_lir_ast_document(p: &Program) -> Result<String, serde_json::Error> {
    let doc = LirAstDocument {
        schema_version: AST_JSON_SCHEMA_VERSION,
        program: p.clone(),
    };
    serde_json::to_string_pretty(&doc)
}

/// Parse JSON envelope and return the program. Fails if `schema_version` is not supported.
pub fn deserialize_lir_ast_document(s: &str) -> Result<Program, AstJsonError> {
    let doc: LirAstDocument = serde_json::from_str(s).map_err(AstJsonError::Json)?;
    if doc.schema_version != AST_JSON_SCHEMA_VERSION {
        return Err(AstJsonError::UnsupportedSchemaVersion {
            got: doc.schema_version,
        });
    }
    Ok(doc.program)
}
