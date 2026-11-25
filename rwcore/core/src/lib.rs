//! RWCore C1.1: bounded, read-only DOCX package graph.
#![forbid(unsafe_code)]

pub mod model;
pub mod opc;
pub mod package;
pub mod zip_guard;
pub use model::{Block, Document, Paragraph, Run, Diagnostic, InlinePictureAt, RunContent};
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

pub mod table_properties;
pub use table_properties::{Appearance, TableWidth, TableEdge, TableColor, TableBorder};

pub use table_properties::{TableShading, TableLayout, CellVerticalAlign, RowHeight, RowHeightRule};

pub mod table_effective;
pub use table_effective::{resolve_effective_cell, resolve_preferred_width, EffectiveCell, PreferredWidth, TableDirection, PropertySource, Sourced};

pub mod table_borders;
pub use table_borders::{TableBorderPlan, ThreeLineBorders};

pub mod table_geometry;
pub use table_geometry::{compute_table_geometry, compute_declared_grid_geometry, TableGeometry, CellGeometry, CellInsets};

pub mod table_rows;
pub use table_rows::{layout_table_rows, place_vertical, TableRowLayout, CellMeasurement, VerticalPlacement};

pub mod table_columns;
pub use table_columns::{allocate_table_columns, TableColumnLayout, CellWidthMeasurement, CellWidthResult};

pub mod table_layout;
pub use table_layout::{layout_nested_table, TableMeasurer, NestedLayoutOptions, NestedTableLayout, NestedCellLayout, NestedBlockLayout};

pub mod media;
pub use media::{load_media, MediaLimits, MediaCatalog, MediaResource, MediaBinding, ImageFormat};

pub mod image_header;
pub use image_header::{parse_image_dimensions, ImageDimensions, ImageLimits};

pub mod drawing_inline;
pub use drawing_inline::{parse_inline_picture, emu_to_twips, PictureExtent, PictureIdentity, InlinePicture, InlineMediaBindings};

pub mod drawing_anchor;
pub use drawing_anchor::{parse_floating_picture, FloatingPicture, AnchorDistances, RelativeFrom, AnchorAlignment, PositionValue, AnchorPosition, AnchorWrap, WrapSide};

pub mod drawing_layout;
pub use drawing_layout::{EmuSpan, EmuRect, InsideEdge, AnchorLayoutContext, FloatingPlacement, ContourPolicy, LineWrap, place_floating_picture, wrap_floating_line};

pub mod picture_geometry;
pub use picture_geometry::{parse_picture_geometry, PictureCrop, PictureTransform, PictureGeometry, PictureMapping, GeometryPoint, GeometryBounds};

pub mod drawing_integration;
pub use drawing_integration::{PictureGeometryState, RunDrawingKind, RunDrawingAt, PictureKindRef, PictureAtRef, PictureOccurrence, DocumentWithMedia, read_docx_with_media};

pub mod edit;
pub use edit::{CellStep,RunPath,TextUnit,text_byte_offset,text_offset_from_byte,InsertAffinity,InteriorAnchorPolicy,TextEdit,EditLimits,EditReport,apply_text_transaction};

pub mod selection;
pub use selection::{ParagraphPath,SelectionLimits,RunBoundary,RunPoint,BytePoint,Selection,ResolvedSelection,ParagraphIndex,SelectionEdit,replace_selection};

pub mod history;
pub use history::{HistoryLimits,HistoryUsage,HistoryReport,HistoryEditReport,StructureHistoryReport,DocumentEditor};

pub mod structure;
pub use structure::{SplitAnchors,SplitPoint,JoinFormatting,StructureOp,StructureTransaction,StructureLimits,StructureReport,apply_structure_transaction};

pub mod font;
pub use font::{FontTag,FontFlavor,FontContainerKind,FontLimits,FontTableRecord,FontFace,FontContainer,parse_font};

pub mod font_metrics;
pub use font_metrics::{FontBounds,FontHead,TrueTypeMaxima,FontMaxp,HorizontalHeader,GlyphHorizontalMetric,FontMetricLimits,FontMetrics,FontScale,parse_font_metrics};

pub mod font_cmap;
pub use font_cmap::{CmapFormat,CmapSource,CmapLimits,UnicodeCmap,parse_unicode_cmap};
