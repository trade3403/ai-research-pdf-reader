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

pub mod font_measure;
pub use font_measure::{MissingGlyphPolicy,MeasureUnit,MeasureLimits,MeasureOptions,NominalGlyph,NominalLineMetrics,TextMeasurement,NominalFont,parse_nominal_font};

pub mod font_outline;
pub use font_outline::{OutlineLimits,GlyphOutlineKind,OutlinePoint,HalfPoint,OutlineCommand,SimpleGlyph,TrueTypeOutlines,parse_true_type_outlines};

pub mod font_composite;
pub use font_composite::{COMPOSITE_ONE,ComponentPlacement,ComponentMatrix,GlyphComponent,CompositeGlyph,CompositeLimits,FixedOutlinePoint,AssembledGlyph,parse_composite_glyph,assemble_true_type_glyph};

pub mod font_path;
pub use font_path::{PATH_ONE,PathLimits,PathPoint,GlyphPathCommand,GlyphPathBounds,GlyphPath,build_glyph_path,true_type_glyph_path};

pub mod font_flatten;
pub use font_flatten::{FlattenLimits,FlattenOptions,ContourOrientation,FlatContour,FlattenedGlyph,flatten_glyph_path};

pub mod font_raster;
pub use font_raster::{GlyphFillRule,RasterTransform,RasterViewport,RasterLimits,RasterOptions,GlyphBitmap,rasterize_flat_glyph};

pub mod font_render;
pub use font_render::{RenderFontLimits,GlyphRenderLimits,GlyphRenderOptions,RenderedGlyph,TrueTypeRenderer,parse_true_type_renderer};

pub mod font_text;
pub use font_text::{TextRenderLimits,TextRenderOptions,PlacedGlyph,RenderedText,render_nominal_text};

pub mod font_compare;
pub use font_compare::{BitmapComparison,compare_glyph_bitmaps};

pub mod font_coverage;
pub use font_coverage::{GlyphCoverage,parse_glyph_coverage};

pub mod font_gsub;
pub use font_gsub::{GsubRecord,GsubDirectory,GsubLimits,parse_gsub_directory};

pub use font_gsub::{GsubLangSys,GsubScript,GsubFeature,GsubLookup,GsubLayout,parse_gsub_layout};

pub mod font_single_subst;
pub use font_single_subst::{SingleSubstLimits,SingleSubstitution,parse_single_substitution};

pub mod font_multiple_subst;
pub use font_multiple_subst::{MultipleSubstLimits,MultipleSubstitution,parse_multiple_substitution};

pub mod font_alternate_subst;
pub use font_alternate_subst::{AlternateSubstLimits,AlternateSubstitution,parse_alternate_substitution};

pub mod font_ligature_subst;
pub use font_ligature_subst::{LigatureLimits,LigatureRule,LigatureMatch,LigatureSubstitution,parse_ligature_substitution};

pub mod font_context_subst;
pub use font_context_subst::{ContextLimits,ContextAction,ContextRule,ContextMatch,ContextSubstitution,parse_context_substitution};

pub mod font_class_def;
pub use font_class_def::{GlyphClassDef,parse_glyph_class_def};
pub use font_context_subst::{ClassContextSubstitution,parse_class_context_substitution};

pub use font_context_subst::{CoverageContextSubstitution,parse_coverage_context_substitution};

pub mod font_chain_context_subst;
pub use font_chain_context_subst::{ChainContextRule,ChainContextSubstitution,parse_chain_context_substitution};

pub use font_chain_context_subst::{ClassChainContextSubstitution,parse_class_chain_context_substitution};

pub use font_chain_context_subst::{CoverageChainContextSubstitution,parse_coverage_chain_context_substitution};

pub mod font_extension_subst;
pub use font_extension_subst::{ExtensionSubstitution,ExtensionTarget,ExtensionLimits,parse_extension_substitution,resolve_extension_lookup,parse_extension_target,parse_extension_target_substitution,parse_direct_gsub_target};

pub mod font_reverse_subst;
pub use font_reverse_subst::{ReverseSubstLimits,ReverseSubstitution,parse_reverse_substitution};

pub mod font_gdef;
pub use font_gdef::{GdefLimits,GdefDirectory,GdefGlyphClass,Gdef,parse_gdef_directory,parse_gdef,parse_gdef_with_mark_set_limits};

pub mod font_gdef_mark_sets;
pub use font_gdef_mark_sets::{MarkGlyphSetLimits,MarkGlyphSets,parse_mark_glyph_sets};

pub mod font_lookup_filter;
pub use font_lookup_filter::{LookupFilter,LookupFilterLimits,GlyphEligibility,LOOKUP_RIGHT_TO_LEFT,LOOKUP_IGNORE_BASE,LOOKUP_IGNORE_LIGATURE,LOOKUP_IGNORE_MARK,LOOKUP_USE_MARK_SET,LOOKUP_MARK_ATTACHMENT_MASK};

pub mod font_gsub_plan;
pub use font_gsub_plan::{GsubPlanLimits,GsubPlannedLookup,GsubPlan,build_gsub_base_plan};

pub mod font_gsub_compile;
pub use font_gsub_compile::{GsubCompileLimits,CompiledGsubSubtable,CompiledGsubLookup,compile_gsub_lookups,compile_gsub_plan};

pub mod font_gsub_dependencies;
pub use font_gsub_dependencies::{GsubDependencyLimits,GsubDependencySite,GsubDependency,discover_gsub_dependencies};

pub mod font_gsub_closure;
pub use font_gsub_closure::{GsubClosureLimits,GsubDependencyClosure,analyze_gsub_dependency_closure};

pub use font_gsub_compile::{GsubProgramLimits,CompiledGsubProgram,compile_gsub_program,compile_gsub_plan_program};

pub mod font_gsub_execute;
pub use font_gsub_execute::{GsubSingleExecLimits,GsubSingleExecResult,execute_gsub_single};

pub use font_gsub_execute::{GsubMixedExecLimits,GsubMixedExecResult,execute_gsub_mixed};

pub use font_gsub_execute::{GsubAlternateSelection,execute_gsub_with_alternates};

pub use font_gsub_execute::{GsubLigatureExecLimits,GsubLigatureExecResult,execute_gsub_with_ligatures};

pub mod font_gsub_context_execute;
pub use font_gsub_context_execute::{GSUB_CONTEXT_MAX_DEPTH,GsubContextExecLimits,GsubContextExecResult,execute_gsub_context};

pub use font_gsub_context_execute::execute_gsub_context_with_alternates;

pub use font_gsub_context_execute::{GsubExpansionExecLimits,GsubExpansionExecResult,execute_gsub_context_with_expansion};

pub use font_gsub_context_execute::{GsubContractionExecLimits,GsubContractionExecResult,execute_gsub_context_with_contraction};

pub use font_gsub_context_execute::execute_gsub_chained_context;

pub use font_gsub_context_execute::execute_gsub_chained_context_with_edits;

pub use font_gsub_context_execute::execute_gsub_program;

pub use font_gsub_context_execute::{GsubProgramExecLimits,GsubProgramExecResult};

pub mod font_feature_variations;
pub use font_feature_variations::{FeatureVariationLimits,VariationAxisRange,FeatureVariationCondition,AlternateFeatureTable,FeatureVariationRecord,FeatureVariations,parse_feature_variations,parse_gsub_feature_variations};

pub use font_feature_variations::{GsubVariationPlan,build_gsub_variation_plan};
pub use font_gsub_compile::compile_gsub_variation_plan_program;

pub mod font_fvar;
pub use font_fvar::{VARIATION_FIXED_ONE,VARIATION_F2DOT14_ONE,FvarLimits,FvarAxis,FvarInstance,Fvar,parse_fvar,normalized_fixed_to_f2dot14};

pub mod font_avar;
pub use font_avar::{AvarLimits,AvarPair,AvarSegmentMap,Avar,parse_avar,normalize_variation_coordinates,normalize_tagged_variation_coordinates};

pub mod font_variable_gsub;
pub use font_variable_gsub::{VariableGsubLimits,VariationUserCoordinates,VariableGsubRequest,VariableGsubResult,VariableGsubFace,parse_variable_gsub_face};
