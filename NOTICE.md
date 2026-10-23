# SSBU Tournament Mode — notices

Licensed under the GNU Affero General Public License v3.0 or later (see `LICENSE`).

## CSS-first scene order

Character-select-before-stage-select logic was ported from the standalone
`css_first` plugin (same author), itself adapted from HewDraw Remix (AGPL-3.0):

https://github.com/HDR-Development/HewDraw-Remix

Do not load `libcss_first.nro` or full HDR alongside this plugin — the same
scene-transition hooks would double-apply.
