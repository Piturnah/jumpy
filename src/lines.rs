//! Plugin for easily drawing lines in the world for debugging or editor visualization.

use crate::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub struct LinesPlugin;

impl Plugin for LinesPlugin {
    fn build(&self, app: &mut App) {
        // WIP
        app.add_plugin(ShapePlugin);
    }
}
