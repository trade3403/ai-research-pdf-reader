//! RWCore C1.1: bounded, read-only DOCX package graph.
#![forbid(unsafe_code)]

pub mod model;
pub mod opc;
pub mod package;
pub mod zip_guard;
pub use model::{Block, Document, Paragraph, Run, Diagnostic};
pub use opc::{read_docx, Error, Limits};

pub use package::{inspect_package, resolve_target, PackageGraph, Relationship, ResourceKind, Target};

pub use zip_guard::validate_zip;

pub mod formatting;
pub use formatting::{resolve_formatting, EffectiveFormatting, FormattedBlock, FormattedParagraph, FormattedRun};

pub mod numbering;
pub use numbering::{parse_numbering, NumberingCatalog, NumberingLabel, NumberingCounter, NumberedBlock, NumberedParagraph};

pub mod section;
pub use section::{parse_section, SectionProperties, DocumentSection, SectionEnd};

pub mod geometry;
pub use geometry::{compute_page_geometry, PageGeometry, Rect, GutterSide};

pub mod stories;
pub use opc::read_docx_with_stories;
pub use stories::{Story, StoryCatalog, parse_story, parse_page_settings, PageSettings};

pub use stories::{StoryBinding, PageContext, SectionStories, SelectedStories};

pub mod fields;
pub use fields::{StoryField, FieldKind, FieldEncoding};

pub mod page_numbers;
pub use page_numbers::{PageNumberPlan, PageNumberContext, evaluate_page_field};

pub mod page_structure;
pub use page_structure::{PageStructurePlan, StructuredPage, EvaluatedField};

pub mod table;
pub use table::{parse_table, Table, TableRow, TableCell, VerticalMerge};

pub mod table_grid;
pub use table_grid::{resolve_table_grid, LogicalTable, LogicalCell, MergeAnchor, CellAddress};
