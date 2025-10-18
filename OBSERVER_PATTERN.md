# Observer Pattern Reference (Bevy 0.17)

## Core Principle

**Observers are LOCAL and SELF-CONTAINED** - capture data in the closure, don't query for it!

## ✅ CORRECT Pattern

```rust
let count = 5;
parent.spawn((
    Button,
    Node { ... },
    BackgroundColor(NORMAL_BUTTON),
    MyButtonMarker { count }, // Just a marker/tag for debugging
    observe(move |_: On<Activate>,
                  mut writer: MessageWriter<MyMessage>,
                  some_resource: Res<SomeResource>| {
        // Use captured 'count' directly!
        writer.write(MyMessage {
            data: count,  // ← Captured in closure with 'move'
        });
    }),
    children![(Text::new(format!("+{}", count)), ...)],
))
```

## ❌ WRONG Pattern (DON'T DO THIS)

```rust
// BAD: Querying for button component when you already have the data!
observe(|_activate: On<Activate>,
        button_query: Query<&MyButton>,  // ← Unnecessary!
        mut writer: MessageWriter<MyMessage>| {
    if let Ok(button) = button_query.get(_activate.entity) {
        writer.write(MyMessage { data: button.data });
    }
})
```

**Why it's wrong:** `On<Activate>` already tells you THIS button was clicked. Just capture the data you need in the closure!

## Key Points

1. **On<Event>** - The event knows which entity triggered it
2. **move** - Capture variables from the surrounding scope
3. **Component markers** - Keep them for type safety/debugging, but don't query them in the observer
4. **Delete old systems** - Remove the `Query<(&Interaction, &MyButton), Changed<Interaction>>` systems entirely

## Real Examples from Codebase

### Button with captured data (transport.rs:353)
```rust
observe(move |value_change: On<ValueChange<f32>>,
              mut adjust_writer: MessageWriter<TransportAdjustAllocation>| {
    adjust_writer.write(TransportAdjustAllocation {
        nation,      // ← Captured
        commodity,   // ← Captured
        requested: value_change.value.round() as u32,
    });
})
```

### Button with helper function (diplomacy.rs:76)
```rust
fn execute_diplomatic_action(action: DiplomaticAction) -> impl Bundle {
    observe(move |_: On<Activate>,
                  selection: Res<DiplomacySelection>,
                  player: Option<Res<PlayerNation>>,
                  nation_ids: Query<&NationId>,
                  mut orders: MessageWriter<DiplomaticOrder>| {
        // action is captured from function parameter
        let order = match action { ... };
        orders.write(order);
    })
}
```

## Migration Checklist

1. ✅ Identify button spawn location
2. ✅ Capture needed data in `move` closure
3. ✅ Add `observe()` bundle with `On<Activate>`
4. ✅ Write message directly using captured data
5. ✅ Delete old `handle_*_button_clicks` system
6. ✅ Remove system from plugin registration

## Completed Migrations

### Civilian UI Buttons (src/civilians/ui_components.rs)
- ✅ BuildDepotButton
- ✅ BuildPortButton
- ✅ ImproveTileButton
- ✅ RescindOrdersButton

**Systems removed:** `handle_order_button_clicks`, `handle_improver_button_clicks`, `handle_rescind_button_clicks`

### Diplomacy UI (src/ui/diplomacy.rs)
- ✅ OfferResponseButton (Accept/Decline buttons on diplomatic offers)

**System removed:** `handle_offer_response_buttons`

### Dialog UI (src/ui/city/dialogs/window.rs)
- ✅ DialogCloseButton (X button on building dialogs)

**System removed:** `handle_dialog_close_buttons`

### Workforce UI (src/ui/city/workforce.rs)
- ✅ RecruitWorkersButton (already using observers via allocation macros)
- ✅ TrainWorkerButton (already using observers via allocation macros)
- ✅ HireCivilianButton (dead code removed - no UI actually spawns these yet)

**System removed:** `handle_hire_button_clicks` (dead code)
