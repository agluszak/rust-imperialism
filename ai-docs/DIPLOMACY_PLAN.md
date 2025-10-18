# Diplomacy Implementation Plan

This plan distils the core diplomacy mechanics from the original *Imperialism* manual and adapts them to the current Rust/Bevy codebase. It assumes the present game lacks any AI-controlled diplomatic behaviour or background processing for non-player nations.

## 1. Feature Targets

1. **World overview & information tools** – Provide the player with per-nation intelligence (military strength, economy, relationships, treaty status, trade policies, council votes) before they issue orders.【F:manual_text.txt†L2634-L2710】
2. **Player-driven diplomatic actions** – Allow issuing orders that resolve at end-of-turn: declarations of war/peace, consulates, embassies, non-aggression pacts, alliances, requests for annexation, and foreign aid.【F:manual_text.txt†L2720-L2869】
3. **Trade policy controls** – Enable subsidies/boycotts toward nations with consulates, mirroring manual behaviour.【F:manual_text.txt†L2890-L2937】
4. **Diplomatic offers inbox** – Surface incoming offers (alliances, aid, war demands) and require player responses before advancing turns.【F:manual_text.txt†L2945-L2995】
5. **Relationship tracking & effects** – Maintain persistent relationship scores and treaty states that unlock actions and influence AI decisions.【F:manual_text.txt†L2634-L2803】

## 2. ECS Data Model

| Concept | Representation | Notes |
| --- | --- | --- |
| `NationId` | Existing component | Already used across economy modules.
| `DiplomaticStatus` | `Component` on a nation entity storing treaties per other nation (`HashMap<NationId, TreatyState>`). | Tracks current war/peace, pacts, alliances.【F:manual_text.txt†L2720-L2847】
| `RelationshipScore` | `Resource` or component storing per-pair float plus trend flags. | Values drive unlocks (consulate → embassy → pact → annex).【F:manual_text.txt†L2634-L2819】
| `DiplomaticOrders` | Message queue (resource) containing player-issued commands. | Processed during `Processing` phase.
| `DiplomaticOffers` | Resource storing incoming prompts requiring UI confirmation. | Populated during `Processing`.
| `TradePolicy` | Component/resource per nation pair (subsidy/boycott/neutral). | Requires consulate.
| `ForeignAidLedger` | Resource tracking one-time and locked grants. | Influences relationship trend.【F:manual_text.txt†L2850-L2889】

### Data Flow

1. **PlayerTurn** – UI writes `DiplomaticOrders` (e.g., `OfferAlliance { target }`).
2. **Processing** – Systems resolve orders, mutate treaty/relationship data, and push `DiplomaticEvent`s (for the log/UI).
3. **EnemyTurn** – Future AI will populate `DiplomaticOffers` and plan new orders.
4. **TurnStart** – Clear expired offers, apply ongoing grant payments, decay relationship modifiers.

## 3. Player UI & Interaction

1. **Screen layout** – Follow existing mode overlay pattern (`GameMode::Diplomacy`). Include:
   - Left tabs for *Information*, *Relationships*, *Trade Policies*, *Council* mirrors manual’s layout.【F:manual_text.txt†L2652-L2710】
   - Right tabs for *Overtures*, *Grants*, *Trade Policies*, *Offers*.
2. **Information tab** – Query ECS for selected nation and display: empire size, military strength, favourite trade partner, top exports, treaties. Requires systems aggregating metrics from existing components.
3. **Relationships map** – Colour-map nations by relationship tier (hostile → allied). Provide legend and text values.
4. **Treaty overlay** – Render icons/lines indicating wars, alliances, consulates, embassies.
5. **Orders workflow** – Selecting an action highlights valid targets and shows cost (e.g., $500 for consulate, $5000 for embassy). Permit cancellation before end turn.【F:manual_text.txt†L2720-L2834】
6. **Incoming offers modal** – When `DiplomaticOffers` non-empty, present dialog queue the player must accept/decline prior to ending the turn.【F:manual_text.txt†L2945-L2995】

## 4. Unlock & Progression Logic

1. **Baseline** – All nations start at peace, with neutral relations (~0).
2. **Consulates** – Require neutral or better relations and $500. Unlocks trade policy controls and enables relationship gain from trades.【F:manual_text.txt†L2768-L2779】
3. **Embassies** – Require consulate, relationship threshold (e.g., ≥30), and $5000. Unlocks pacts, aid, entry for civilians, annex requests.【F:manual_text.txt†L2780-L2790】
4. **Non-aggression pact** – Free, requires embassy. Grants relationship boost and prevents offensive orders unless broken.【F:manual_text.txt†L2794-L2803】
5. **Alliances** – Great Powers only, require embassy and positive relations (≥40). Establish mutual defence triggers; refusal hurts trust.【F:manual_text.txt†L2821-L2843】
6. **Join Empire** – Available when relationship ≥70 (Minor Nations) or target is desperate (Great Powers). Success converts to `Colony` status, transferring assets.【F:manual_text.txt†L2804-L2849】
7. **Foreign aid & subsidies** – Select amount ($100–$10,000) per turn; locked grants auto-repeat. Increase relationship over time proportional to cumulative spend.【F:manual_text.txt†L2850-L2931】
8. **Trade policies** – With consulate, allow subsidy/boycott toggles affecting trade resolution and relationships.【F:manual_text.txt†L2890-L2937】

## 5. Relationship Mechanics

1. **Score range** – Use `-100..=100` with named bands (`Hostile`, `Unfriendly`, `Neutral`, `Cordial`, `Warm`, `Allied`).
2. **Events** – Adjust scores based on actions: war declarations, aid, trades, treaties, betrayals. Derive modifiers from manual descriptions (e.g., breaking pacts causes global penalty).【F:manual_text.txt†L2634-L2995】
3. **Decay** – Each turn, drift toward neutral while preserving locked modifiers (pacts, aid). This models “benefits build over time”.【F:manual_text.txt†L2639-L2889】
4. **Visibility** – Player sees numeric & qualitative values; AI uses thresholds to decide offers in future.

## 6. Turn Resolution Rules

1. **Order queue resolution** – Evaluate overtures in deterministic priority: peace/war → treaties → consulates/embassies → aid → annexation. Prevent contradictory states.
2. **Simultaneous execution** – When multiple nations act, apply to cached state then commit to maintain fairness. Use reservation-like approach mirroring resource allocations.
3. **War consequences** – On declaration, set `War` status, reduce relationship by large amount, adjust global modifiers based on victim’s friends/enemies.【F:manual_text.txt†L2733-L2754】
4. **Peace** – Available once war active; acceptance conditions determined by future AI heuristics. For now, allow player to force acceptance for Minor Nations; Great Powers accept only if simulated evaluation deems favourable (placeholder: check comparative military strength & capital threat).【F:manual_text.txt†L2752-L2756】
5. **Council votes** – Track colonies and allied influence to integrate with victory conditions later.【F:manual_text.txt†L2804-L2815】

## 7. AI Placeholder Strategy

Because no AI framework exists yet:

1. **Deterministic scripts** – For MVP, define rule-based reactions (e.g., auto-accept consulates, embassies) mirroring manual assurances.【F:manual_text.txt†L2768-L2799】
2. **Future hooks** – Provide trait `DiplomacyAgent` with callbacks (`evaluate_offer`, `plan_orders`) so AI modules can plug in later.
3. **Simulation stubs** – Implement simple evaluation functions (trade value, military strength) to power acceptance checks.
4. **Testing harness** – Add unit tests for acceptance logic and order resolution without needing full AI opponents.

## 8. Implementation Phases

1. **Scaffolding** – Define data components/resources, register systems in `src/lib.rs`, ensure serialization for saves.
2. **UI foundation** – Build diplomacy screen overlay with tab navigation; integrate with existing Bevy UI patterns.
3. **Order handling** – Implement message structs and processing systems for each overture type.
4. **Relationship engine** – Create modifier framework (per-turn adjustments, event-driven deltas).
5. **Offer inbox** – Build UI + logic for incoming offers, log events, and blocking end-turn until resolved.
6. **Trade policy integration** – Connect subsidies/boycotts to market logic once Market v2 work begins.
7. **Testing & balancing** – Write unit/integration tests for order resolution, relationship transitions, and UI state. Add debug visualization for relationships to speed tuning.

## 9. Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Complexity explosion without AI | Keep MVP deterministic, deferring dynamic AI decisions until dedicated system exists. |
| Data duplication between treaties & relationships | Encapsulate in a single `DiplomaticState` struct per nation pair. |
| UI clutter | Follow manual’s tabbed layout and reuse existing UI components. |
| Save-game compatibility | Version structs and add migration path early. |
| Integration with victory conditions | Expose colony/alliance changes through events to update council logic. |

## 10. Success Criteria

- Player can inspect any nation, view relationships, and issue diplomatic orders each turn.
- Orders resolve consistently at end of turn and update treaties/relationships.
- Incoming offers block end turn until decisions are made.
- Relationship tiers gate advanced actions (embassy, pact, alliance, annex).
- Systems emit events for UI/log and are test-covered.

