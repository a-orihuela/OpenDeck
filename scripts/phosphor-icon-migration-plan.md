# Phosphor Icon Migration (OpenDeck)

## Scope

- Built-in action icons use static SVG files in `static/builtin`.
- UI component icons can keep using `phosphor-svelte` components.
- This plan covers static SVG replacement for built-in action icons.

## Inputs Prepared

- Mapping file: `scripts/phosphor-icon-map.json`
- Source style: Phosphor `duotone`

## Phase 1 (next): Generator Script

1. Create `scripts/generate_phosphor_svgs.ts`.
2. Read `scripts/phosphor-icon-map.json`.
3. For each mapped icon:
   - Resolve icon source from Phosphor package/export.
   - Convert to static SVG output.
   - Normalize:
     - `viewBox=0 0 256 256`
     - preserve duotone layers
     - set secondary opacity to `0.2`
4. Write output into `static/builtin/<filename>.svg`.

## Phase 2: Pilot Batch (5 icons)

Generate and apply first:

1. `run-command.svg`
2. `open-url.svg`
3. `simulate-input.svg`
4. `switch-profile.svg`
5. `pomodoro.svg`

## Phase 3: Full Replacement

Apply generation to all mapped icons, including state icons:

- `volume-on.svg` / `mute.svg`
- `play.svg` / `pause.svg`
- `folder.svg` / `folder-close.svg`

## Validation Checklist

1. Action list thumbnails render correctly.
2. Key/encoder rendering still works in preview and on device.
3. Toggle state icons switch correctly.
4. Fallback icons (`alert.svg`, `ok.svg`) still show in renderer flows.
5. No missing icon 404s in webview.

## Notes

- No runtime icon font is required for built-in actions.
- Keeping static SVG preserves compatibility with image rendering pipeline.
