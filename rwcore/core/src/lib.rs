//! RWCore C1.0: bounded, read-only DOCX bootstrap.
#![forbid(unsafe_code)]

pub mod model;
pub mod opc;
pub use model::{Block, Document, Paragraph, Run, Diagnostic};
pub use opc::{read_docx, Error, Limits};
