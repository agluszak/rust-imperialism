# Civilian Unit Expansion Design

## Goals
- Introduce **Prospector**, **Miner**, **Rancher**, and **Forester** civilians alongside the existing Engineer and Farmer units.
- Ensure civilian systems (selection, orders, jobs, rendering, UI) remain data-driven and extensible for additional unit types.
- Support the gameplay loop described in the Imperialism manual: prospecting hidden minerals, opening mines, and improving agricultural, pastoral, and timber resources.【F:manual_text.txt†L961-L1036】

## Design Overview
We will describe each civilian type with a static definition that drives:
- Available action buttons (e.g. Prospect, Improve Tile, Build Depot/Port).
- Whether the unit can improve tile resources and, if so, the predicate that validates eligible resources.

`CivilianKind::definition()` will return a `CivilianKindDefinition` containing:
- `display_name`: panel heading text.
- `action_buttons`: collection of `CivilianActionButton { label, order }` used to build UI.
- `resource_predicate`: optional function pointer `fn(&TileResource) -> bool` used by improvement systems to validate tiles.

This replaces the scattered `matches!` checks and manual UI panels with generic logic that works for any unit type described in the metadata.

## Systems Impacted
1. **UI (`ui_components.rs`)**
   - Replace specialized Engineer/Improver panels with a single `CivilianOrdersPanel` that reads button descriptors from the selected unit's definition. This keeps UI logic consistent and makes future units trivial to add.
2. **Order Execution (`engineering.rs`)**
   - Update the improvement system to consult `resource_predicate` instead of hard-coded matches, ensuring Miners, Ranchers, and Foresters automatically inherit the correct validation rules. Prospector orders remain immediate but benefit from shared button plumbing.
3. **Types (`types.rs`)**
   - Add metadata structs and helper methods. Provide utility methods like `supports_improvements()` derived from metadata for reuse in systems and UI.
4. **Spawning (`map/province_setup.rs`)**
   - Spawn a starter roster (Engineer, Prospector, Farmer, Miner, Rancher, Forester) for the player near the capital so the new functionality is immediately accessible.
5. **Rendering**
   - No structural changes required; visuals already use `CivilianKind` assets and will automatically work with new units.
6. **Testing**
   - Add coverage ensuring metadata exposes expected buttons and that miners respect resource predicates.

## Gameplay Flow
1. **Prospector** discovers hidden minerals/oil on eligible tiles before extraction is possible.【F:manual_text.txt†L995-L1013】
2. **Miner** opens mines and upgrades them, following the resource development table (Lv0→Lv3 output).【F:manual_text.txt†L1014-L1036】
3. **Rancher** and **Forester** improve livestock/wool and timber resources respectively, boosting output across development levels.【F:manual_text.txt†L1037-L1073】

By encoding these relationships in metadata, the same systems can schedule improvement jobs, render UI, and validate actions without further branching, satisfying the request for generic civilian management.
