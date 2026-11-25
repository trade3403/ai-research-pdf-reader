# Outstanding work and limits

- CI confirms synthetic regressions and compilation, not full Microsoft Word fidelity or real-world performance. Legal real DOCX paper corpus, Word reference comparisons, dependency security/license audit and exact lock artifact provenance are outstanding.
- No GUI, installation package, actual renderer or visual acceptance yet; no claim otherwise.
- Font metrics/shaping and intrinsic nested content measurement still external. Row/column allocation is documented deterministic subset; no complete Word pagination, table style cascade, border painting, distributed vertical placement or RTL reordering.
- Header/footer nested tables and unsupported wrappers remain opaque. Inline images in supported runs preserve order, but invalid/unsupported drawing nodes are diagnosed and omitted, not round-trip preserved. AlternateContent, floating anchor document integration and full floating layout, image-aware line breaking and actual painting still missing.
- Raw media signature and header parsing do not validate complete files, decode pixels, process ICC/DPI/orientation or support SVG/EMF/WMF. External links inert.
- Run public text mutation can invalidate image offsets; inline_content checks boundaries/order but later C3 editing transactions must maintain semantic anchor positions.
- DOCX writing, full round-trip fidelity, actual page counts, formulas, cross references, Electron integration not implemented by current C phase.
- RDHS cannot read precise remaining account/context quota. Fixed handoff checkpoints are preventive, not system limit detection. Branch CAS protects writes but does not replace coordination of old/new windows and existing automation.

- C2.4d-1 verified CI #55: standalone floating metadata only; wrap polygons, effect extents, unknown optional extensions, full schema validation, placement (in that batch), collision handling, painting and document-tree integration are not implemented. Simple position absent defaults to false; numeric values use a conservative integer subset. No real Word fidelity claim.

- C2.4d-2 pending CI: signed EMU static placement and single-anchor rectangular avoidance only. Inside edges, measured reference spans and cell context are caller responsibilities. Largest-side ties choose physical left; center uses floor rounding; no clipping. Tight/through polygons are not parsed; fallback must be explicitly enabled. No full line breaking, multi-object collision/flow, actual cell constraint, effect geometry, painter, pagination or Word-reference comparison.
