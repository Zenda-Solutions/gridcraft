# Zenda build of GridCraft for Work

Based on upstream v0.13.0 (43284377). Feature `zenda` adds a bounded `work.events`
queue, the orange Theo mascot button and Share through Work. Upstream builds keep
stock behavior. No document format changes. Work owns accounts and cloud sending.
The tool emits metadata only; unsaved or dirty workbooks must be saved in Work first.

No startup/manual upstream update requests in this build. Discord is absent from
About. `zenda-theo-panel` is still pending in Work, as in Texto.

Theo's image is the existing Zenda Work mascot (`app/ui/theo/theo-icon-64.png`),
SHA-256 pinned in Work's cycle report. Button: #EA6C1E with black text, keyboard
focus and localized AccessKit name (es/en/pt-BR).

Packaging: `pwsh zenda/package.ps1 -Z 1`; workflow `Zenda` publishes immutable
`zenda-v0.13.0-z1` assets. Verify size and SHA-256 before editing Work's manifest.

Translation additions retain existing labels; sources: GridCraft v0.18.0 es/pt catalogs
and craft-i18n 74e77706491d260c868bec6df686f92b90baae38 (MIT OR Apache-2.0). Full catalog coverage remains in the cycle report.

## Checks

`cargo test --config 'profile.dev.package.gridcraft-ui-egui.opt-level=0' -p gridcraft-ui-egui --lib --features zenda zenda`
checks events, toolbar routing, the unsaved workbook, localized accessible names,
upstream update suppression and catalog/placeholder coverage. Run without the
feature too. The package uses the normal release profile; the override only
avoids optimizing hundreds of unrelated UI tests during the quick test run.
Set `ZENDA_THEO_PREVIEW` to a PNG path for an offscreen button preview.

Work's `check_planilla` example replays every lesson's commands in gridcraft-cli
and applies Work's real check code to each response; practice files use a new
temporary directory. It never opens the desktop or uses the person's documents.
