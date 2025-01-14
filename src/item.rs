use crate::{prelude::*, utils::invalid_entity};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        // Pre-initialize components so that the scripting engine doesn't throw an error if a script
        // tries to access the component before it has been added to the world by a Rust system.
        app.world.init_component::<ItemDropped>();
        app.world.init_component::<ItemGrabbed>();
        app.world.init_component::<ItemUsed>();
        app.world.init_component::<Item>();

        app.register_type::<Item>()
            .register_type::<ItemDropped>()
            .register_type::<ItemGrabbed>()
            .register_type::<ItemUsed>()
            .extend_rollback_plugin(|plugin| {
                plugin
                    .register_rollback_type::<Item>()
                    .register_rollback_type::<ItemDropped>()
                    .register_rollback_type::<ItemUsed>()
                    .register_rollback_type::<ItemGrabbed>()
            });
    }
}

/// Component indicating that the entity it is attached to is an item that can be picked up
/// by a player.
#[derive(Component, Reflect, Default, Serialize, Deserialize, Debug)]
#[reflect(Default, Component)]
pub struct Item {
    /// The path to the item's script, or if the item is built-in a string like `core:sword`.
    pub script: String,
}

/// Marker component added to items that have been drop in the current frame.
///
/// This component will be removed from the item at the end of the frame.
#[derive(Component, Reflect, Debug)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct ItemDropped {
    /// The player that dropped the item
    pub player: Entity,
}

impl Default for ItemDropped {
    fn default() -> Self {
        Self {
            player: invalid_entity(),
        }
    }
}

/// Marker component indicating the item has been used this frame.
///
/// This component will be removed from the item at the end of the frame.
#[derive(Component, Reflect, Debug)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct ItemUsed {
    /// The player that dropped the item
    pub player: Entity,
}

impl Default for ItemUsed {
    fn default() -> Self {
        Self {
            player: invalid_entity(),
        }
    }
}

/// Marker component indicating the item has been grabbed this frame.
///
/// This component will be removed from the item at the end of the frame.
#[derive(Component, Reflect, Debug)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct ItemGrabbed {
    /// The player that dropped the item
    pub player: Entity,
}

impl Default for ItemGrabbed {
    fn default() -> Self {
        Self {
            player: invalid_entity(),
        }
    }
}
