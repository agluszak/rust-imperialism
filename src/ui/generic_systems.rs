/// Generic systems that work across different screen marker types
///
/// This module demonstrates Bevy's generic system pattern, allowing a single
/// system implementation to work with multiple component types by using
/// the turbofish syntax when registering the system.
///
/// Example from Bevy documentation:
/// ```text
/// .add_systems(OnExit(GameMode::Market), hide_screen::<MarketScreen>)
/// .add_systems(OnExit(GameMode::City), hide_screen::<CityScreen>)
/// ```
use bevy::prelude::*;

/// Generic system to hide UI screens by setting their visibility to Hidden
///
/// This replaces individual hide_*_screen functions that all did the same thing.
/// Usage: Register with turbofish syntax like `hide_screen::<MarketScreen>`
///
/// **Before (duplicated):**
/// ```text
/// pub fn hide_market_screen(mut roots: Query<&mut Visibility, With<MarketScreen>>) {
///     for mut vis in roots.iter_mut() {
///         *vis = Visibility::Hidden;
///     }
/// }
///
/// pub fn hide_city_screen(mut roots: Query<&mut Visibility, With<CityScreen>>) {
///     for mut vis in roots.iter_mut() {
///         *vis = Visibility::Hidden;
///     }
/// }
/// ```
///
/// **After (generic):**
/// ```text
/// pub fn hide_screen<T: Component>(mut roots: Query<&mut Visibility, With<T>>) {
///     for mut vis in roots.iter_mut() {
///         *vis = Visibility::Hidden;
///     }
/// }
/// ```
pub fn hide_screen<T: Component>(mut roots: Query<&mut Visibility, With<T>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

/// Generic system to despawn UI screens
///
/// This replaces individual despawn_*_screen functions that all did the same thing.
/// Usage: Register with turbofish syntax like `despawn_screen::<TransportScreen>`
///
/// **Before (duplicated):**
/// ```text
/// fn despawn_transport_screen(mut commands: Commands, query: Query<Entity, With<TransportScreen>>) {
///     for entity in query.iter() {
///         commands.entity(entity).despawn();
///     }
/// }
/// ```
///
/// **After (generic):**
/// ```text
/// pub fn despawn_screen<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
///     for entity in query.iter() {
///         commands.entity(entity).despawn();
///     }
/// }
/// ```
pub fn despawn_screen<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Generic system to show UI screens by setting their visibility to Visible
///
/// Can be used when you need to show a screen without spawning new entities.
pub fn show_screen<T: Component>(mut roots: Query<&mut Visibility, With<T>>) {
    for mut vis in roots.iter_mut() {
        *vis = Visibility::Visible;
    }
}
