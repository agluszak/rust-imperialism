use bevy::app::{PluginGroup, PluginGroupBuilder};
use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};
use bevy::render::diagnostic::RenderDiagnosticsPlugin;

pub struct DebugPlugins;

impl PluginGroup for DebugPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // Adds a system that prints diagnostics to the console.
            // The other diagnostics plugins can still be used without this if you want to use them in an ingame overlay for example.
            .add(LogDiagnosticsPlugin::default())
            // Adds frame time, FPS and frame count diagnostics.
            .add(FrameTimeDiagnosticsPlugin::default())
            // Adds an entity count diagnostic.
            .add(EntityCountDiagnosticsPlugin::default())
            // Adds cpu and memory usage diagnostics for systems and the entire game process.
            .add(SystemInformationDiagnosticsPlugin)
            // Forwards various diagnostics from the render app to the main app.
            // These are pretty verbose but can be useful to pinpoint performance issues.
            .add(RenderDiagnosticsPlugin)
            .build()
    }
}
