//! RWCore C1.1: bounded, read-only DOCX package graph.
#![forbid(unsafe_code)]

pub mod model;
pub mod opc;
pub mod package;
pub use model::{Block, Document, Paragraph, Run, Diagnostic};
pub use opc::{read_docx, Error, Limits};

pub use package::{inspect_package, resolve_target, PackageGraph, Relationship, ResourceKind, Target};
