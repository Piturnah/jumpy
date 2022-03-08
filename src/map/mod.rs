use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

mod decoration;
mod sproinger;

pub use decoration::*;
pub use sproinger::*;

use core::prelude::*;
use core::Result;

#[cfg(not(feature = "ultimate"))]
use crate::gui::combobox::ComboBoxValue;

use crate::{
    json::{self, TiledMap},
    Resources,
};

pub type MapProperty = core::json::GenericParam;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBackgroundLayer {
    pub texture_id: String,
    pub depth: f32,
    #[serde(with = "core::json::vec2_def")]
    pub offset: Vec2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(into = "json::MapDef", from = "json::MapDef")]
pub struct Map {
    #[serde(
        default = "Map::default_background_color",
        with = "core::json::ColorDef"
    )]
    pub background_color: Color,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub background_layers: Vec<MapBackgroundLayer>,
    #[serde(with = "core::json::def_vec2")]
    pub world_offset: Vec2,
    pub grid_size: Size<u32>,
    pub tile_size: Size<f32>,
    pub layers: HashMap<String, MapLayer>,
    pub tilesets: HashMap<String, MapTileset>,
    #[serde(skip)]
    pub draw_order: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, MapProperty>,
    #[serde(default, with = "core::json::vec2_vec")]
    pub spawn_points: Vec<Vec2>,
}

impl Map {
    pub const PLATFORM_TILE_ATTRIBUTE: &'static str = "jumpthrough";

    // Padding added to colliders for collision checks since the collision system stops movement
    // before collision is registered, if not.
    pub const COLLIDER_PADDING: f32 = 8.0;

    const FLATTENED_BACKGROUND_PADDING_X: f32 = 100.0;
    const FLATTENED_BACKGROUND_PADDING_Y: f32 = 100.0;

    pub fn new(tile_size: Vec2, grid_size: UVec2) -> Self {
        Map {
            background_color: Self::default_background_color(),
            background_layers: Vec::new(),
            world_offset: Vec2::ZERO,
            grid_size: grid_size.into(),
            tile_size: tile_size.into(),
            layers: HashMap::new(),
            tilesets: HashMap::new(),
            draw_order: Vec::new(),
            properties: HashMap::new(),
            spawn_points: Vec::new(),
        }
    }

    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = load_file(path).await?;
        let map = serde_json::from_slice(&bytes).unwrap();

        Ok(map)
    }

    pub async fn load_tiled<P: AsRef<Path>>(path: P, export_path: Option<P>) -> Result<Self> {
        let bytes = load_file(path).await?;
        let tiled_map: TiledMap = serde_json::from_slice(&bytes).unwrap();

        let map = tiled_map.into_map();

        if let Some(export_path) = export_path {
            map.save(export_path).unwrap();
        }

        Ok(map)
    }

    pub fn get_size(&self) -> Size<f32> {
        Size::new(
            self.grid_size.width as f32 * self.tile_size.width,
            self.grid_size.height as f32 * self.tile_size.height,
        )
    }

    pub fn contains(&self, position: Vec2) -> bool {
        #[cfg(feature = "ultimate")]
        let map_size = self.grid_size.as_vec2() * self.tile_size;
        #[cfg(not(feature = "ultimate"))]
        let map_size = Size::from(UVec2::from(self.grid_size).as_f32()) * self.tile_size;
        let rect = Rect::new(
            self.world_offset.x,
            self.world_offset.y,
            map_size.width,
            map_size.height,
        );
        rect.contains(position)
    }

    pub fn to_grid(&self, rect: &Rect) -> URect {
        let p = self.to_coords(rect.point());
        let w = ((rect.w / self.tile_size.width) as u32).clamp(0, self.grid_size.width - p.x - 1);
        let h = ((rect.h / self.tile_size.height) as u32).clamp(0, self.grid_size.height - p.y - 1);
        URect::new(p.x, p.y, w, h)
    }

    pub fn to_coords(&self, position: Vec2) -> UVec2 {
        let x = (((position.x - self.world_offset.x) / self.tile_size.width) as u32)
            .clamp(0, self.grid_size.width - 1);
        let y = (((position.y - self.world_offset.y) / self.tile_size.height) as u32)
            .clamp(0, self.grid_size.height - 1);
        uvec2(x, y)
    }

    pub fn to_index(&self, coords: UVec2) -> usize {
        ((coords.y * self.grid_size.width) + coords.x) as usize
    }

    pub fn to_position(&self, point: UVec2) -> Vec2 {
        vec2(
            (point.x as f32 * self.tile_size.width) + self.world_offset.x,
            (point.y as f32 * self.tile_size.height) + self.world_offset.y,
        )
    }

    pub fn get_tile(&self, layer_id: &str, x: u32, y: u32) -> &Option<MapTile> {
        let layer = self
            .layers
            .get(layer_id)
            .unwrap_or_else(|| panic!("No layer with id '{}'!", layer_id));

        if x >= self.grid_size.width || y >= self.grid_size.height {
            return &None;
        };

        let i = (y * self.grid_size.width + x) as usize;
        &layer.tiles[i]
    }

    pub fn get_tiles(&self, layer_id: &str, rect: Option<URect>) -> MapTileIterator {
        let rect = rect.unwrap_or_else(|| URect::new(0, 0, self.grid_size.width, self.grid_size.height));
        let layer = self
            .layers
            .get(layer_id)
            .unwrap_or_else(|| panic!("No layer with id '{}'!", layer_id));

        MapTileIterator::new(layer, rect)
    }

    pub fn get_collisions(&self, collider: &Rect, should_ignore_platforms: bool) -> Vec<Rect> {
        let collider = Rect::new(
            collider.x - Self::COLLIDER_PADDING,
            collider.y - Self::COLLIDER_PADDING,
            collider.w + Self::COLLIDER_PADDING * 2.0,
            collider.h + Self::COLLIDER_PADDING * 2.0,
        );

        let grid = self.to_grid(&Rect::new(
            collider.x - self.tile_size.width,
            collider.y - self.tile_size.height,
            collider.w + self.tile_size.width * 2.0,
            collider.h + self.tile_size.height * 2.0,
        ));

        let mut collisions = Vec::new();

        let platform_attr = Self::PLATFORM_TILE_ATTRIBUTE.to_string();

        for layer in self.layers.values() {
            if layer.is_visible && layer.has_collision {
                for (x, y, tile) in self.get_tiles(&layer.id, Some(grid)) {
                    if let Some(tile) = tile {
                        if !(should_ignore_platforms && tile.attributes.contains(&platform_attr)) {
                            let tile_position = self.to_position(uvec2(x, y));

                            let tile_rect = Rect::new(
                                tile_position.x,
                                tile_position.y,
                                self.tile_size.width,
                                self.tile_size.height,
                            );

                            if tile_rect.overlaps(&collider) {
                                collisions.push(tile_rect);
                            }
                        }
                    }
                }
            }
        }

        collisions
    }

    pub fn is_collision_at(&self, position: Vec2, should_ignore_platforms: bool) -> bool {
        let index = {
            let coords = self.to_coords(position);
            self.to_index(coords)
        };

        for layer in self.layers.values() {
            if layer.is_visible && layer.has_collision {
                if let Some(Some(tile)) = layer.tiles.get(index) {
                    return !(should_ignore_platforms
                        && tile
                            .attributes
                            .contains(&Self::PLATFORM_TILE_ATTRIBUTE.to_string()));
                }
            }
        }

        false
    }

    fn background_parallax(texture: Texture2D, depth: f32, camera_position: Vec2) -> Rect {
        let size = texture.size();

        let dest_rect = Rect::new(0., 0., size.width, size.height);
        let parallax_w = size.width * 0.5;

        let mut dest_rect2 = Rect::new(
            -parallax_w,
            -parallax_w,
            size.width + parallax_w * 2.,
            size.height + parallax_w * 2.,
        );

        let parallax_x = camera_position.x / dest_rect.w - 0.3;
        let parallax_y = camera_position.y / dest_rect.h * 0.6 - 0.5;

        dest_rect2.x += parallax_w * parallax_x * depth;
        dest_rect2.y += parallax_w * parallax_y * depth;

        dest_rect2
    }

    pub fn draw_background(&self, rect: Option<URect>, camera_position: Vec2, is_parallax_disabled: bool) {
        let rect = rect.unwrap_or_else(|| URect::new(0, 0, self.grid_size.width, self.grid_size.height));

        draw_rectangle(
            self.world_offset.x,
            self.world_offset.y,
            rect.w as f32 * self.tile_size.width,
            rect.h as f32 * self.tile_size.height,
            self.background_color,
        );

        let resources = storage::get::<Resources>();

        {
            for layer in &self.background_layers {
                let texture_res = resources.textures.get(&layer.texture_id).unwrap();
                let dest_rect = if is_parallax_disabled {
                    #[cfg(feature = "ultimate")]
                    let map_size = self.grid_size.as_vec2() * self.tile_size;
                    #[cfg(not(feature = "ultimate"))]
                    let map_size = Size::from(UVec2::from(self.grid_size).as_f32()) * self.tile_size;

                    let size = texture_res.texture.size();

                    let width = map_size.width + (Self::FLATTENED_BACKGROUND_PADDING_X * 2.0);
                    let height = (width / size.width) * size.height;

                    Rect::new(
                        self.world_offset.x - Self::FLATTENED_BACKGROUND_PADDING_X,
                        self.world_offset.y - Self::FLATTENED_BACKGROUND_PADDING_Y,
                        width,
                        height,
                    )
                } else {
                    let mut dest_rect =
                        Self::background_parallax(texture_res.texture, layer.depth, camera_position);
                    dest_rect.x += layer.offset.x;
                    dest_rect.y += layer.offset.y;
                    dest_rect
                };

                draw_texture(
                    dest_rect.x,
                    dest_rect.y,
                    texture_res.texture,
                    DrawTextureParams {
                        dest_size: Some(Size::new(dest_rect.w, dest_rect.h)),
                        ..Default::default()
                    },
                )
            }
        }
    }

    /// This will draw the map
    pub fn draw<P: Into<Option<Vec2>>>(&self, rect: Option<URect>, camera_position: P) {
        if let Some(camera_position) = camera_position.into() {
            self.draw_background(rect, camera_position, false);
        }

        let rect = rect.unwrap_or_else(|| URect::new(0, 0, self.grid_size.width, self.grid_size.height));

        let mut draw_order = self.draw_order.clone();
        draw_order.reverse();

        let resources = storage::get::<Resources>();
        for layer_id in draw_order {
            if let Some(layer) = self.layers.get(&layer_id) {
                if layer.is_visible && layer.kind == MapLayerKind::TileLayer {
                    for (x, y, tile) in self.get_tiles(&layer_id, Some(rect)) {
                        if let Some(tile) = tile {
                            let world_position = self.world_offset
                                + vec2(x as f32 * self.tile_size.width, y as f32 * self.tile_size.height);

                            let texture_entry =
                                resources.textures.get(&tile.texture_id).unwrap_or_else(|| {
                                    panic!("No texture with id '{}'!", tile.texture_id)
                                });

                            draw_texture(
                                world_position.x,
                                world_position.y,
                                texture_entry.texture,
                                DrawTextureParams {
                                    source: Some(Rect::new(
                                        tile.texture_coords.x, // + 0.1,
                                        tile.texture_coords.y, // + 0.1,
                                        self.tile_size.width,      // - 0.2,
                                        self.tile_size.height,      // - 0.2,
                                    )),
                                    dest_size: Some(self.tile_size),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn get_layer_kind(&self, layer_id: &str) -> Option<MapLayerKind> {
        if let Some(layer) = self.layers.get(layer_id) {
            return Some(layer.kind);
        }

        None
    }

    pub fn default_background_color() -> Color {
        Color::new(0.0, 0.0, 0.0, 1.0)
    }

    #[cfg(any(target_family = "unix", target_family = "windows"))]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    #[cfg(target_family = "wasm")]
    pub fn save<P: AsRef<Path>>(&self, _: P) -> Result<()> {
        Ok(())
    }

    pub fn get_random_spawn_point(&self) -> Vec2 {
        let i = core::rand::gen_range(0, self.spawn_points.len()) as usize;
        self.spawn_points[i]
    }
}

pub struct MapTileIterator<'a> {
    rect: URect,
    current: (u32, u32),
    layer: &'a MapLayer,
}

impl<'a> MapTileIterator<'a> {
    fn new(layer: &'a MapLayer, rect: URect) -> Self {
        let current = (rect.x, rect.y);
        MapTileIterator {
            layer,
            rect,
            current,
        }
    }
}

impl<'a> Iterator for MapTileIterator<'a> {
    type Item = (u32, u32, &'a Option<MapTile>);

    fn next(&mut self) -> Option<Self::Item> {
        let next = if self.current.0 + 1 >= self.rect.x + self.rect.w {
            (self.rect.x, self.current.1 + 1)
        } else {
            (self.current.0 + 1, self.current.1)
        };

        let i = (self.current.1 * self.layer.grid_size.width + self.current.0) as usize;
        if self.current.1 >= self.rect.y + self.rect.h || i >= self.layer.tiles.len() {
            return None;
        }

        let res = Some((self.current.0, self.current.1, &self.layer.tiles[i]));

        self.current = next;

        res
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerKind {
    TileLayer,
    ObjectLayer,
}

impl MapLayerKind {
    pub fn options() -> &'static [&'static str] {
        &["Tiles", "Objects"]
    }
}

#[cfg(not(feature = "ultimate"))]
impl ComboBoxValue for MapLayerKind {
    fn get_index(&self) -> usize {
        match self {
            Self::TileLayer => 0,
            Self::ObjectLayer => 1,
        }
    }

    fn set_index(&mut self, index: usize) {
        *self = match index {
            0 => Self::TileLayer,
            1 => Self::ObjectLayer,
            _ => unreachable!(),
        }
    }

    fn get_options(&self) -> Vec<String> {
        Self::options().iter().map(|s| s.to_string()).collect()
    }
}

impl Default for MapLayerKind {
    fn default() -> Self {
        MapLayerKind::TileLayer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapLayer {
    pub id: String,
    pub kind: MapLayerKind,
    #[serde(default)]
    pub has_collision: bool,
    pub grid_size: Size<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tiles: Vec<Option<MapTile>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<MapObject>,
    #[serde(default)]
    pub is_visible: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, MapProperty>,
}

impl MapLayer {
    pub fn new(id: &str, kind: MapLayerKind, has_collision: bool, grid_size: Size<u32>) -> Self {
        let has_collision = if kind == MapLayerKind::TileLayer {
            has_collision
        } else {
            false
        };

        let mut tiles = Vec::new();
        tiles.resize((grid_size.width * grid_size.height) as usize, None);

        MapLayer {
            id: id.to_string(),
            kind,
            has_collision,
            tiles,
            grid_size,
            ..Default::default()
        }
    }
}

impl Default for MapLayer {
    fn default() -> Self {
        MapLayer {
            id: "".to_string(),
            has_collision: false,
            kind: MapLayerKind::TileLayer,
            grid_size: Size::zero(),
            tiles: Vec::new(),
            objects: Vec::new(),
            is_visible: true,
            properties: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapTile {
    pub tile_id: u32,
    pub tileset_id: String,
    pub texture_id: String,
    #[serde(with = "core::json::vec2_def")]
    pub texture_coords: Vec2,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapObjectKind {
    Item,
    Environment,
    Decoration,
}

impl MapObjectKind {
    const ITEM: &'static str = "item";
    const ENVIRONMENT: &'static str = "environment";
    const DECORATION: &'static str = "decoration";

    pub fn options() -> &'static [&'static str] {
        &["Item", "Environment", "Decoration"]
    }
}

impl From<String> for MapObjectKind {
    fn from(str: String) -> Self {
        if str == Self::ITEM {
            Self::Item
        } else if str == Self::ENVIRONMENT {
            Self::Environment
        } else if str == Self::DECORATION {
            Self::Decoration
        } else {
            let str = if str.is_empty() {
                "NO_OBJECT_TYPE"
            } else {
                &str
            };

            unreachable!("Invalid MapObjectKind '{}'", str)
        }
    }
}

impl From<MapObjectKind> for String {
    fn from(kind: MapObjectKind) -> String {
        match kind {
            MapObjectKind::Item => MapObjectKind::ITEM.to_string(),
            MapObjectKind::Environment => MapObjectKind::ENVIRONMENT.to_string(),
            MapObjectKind::Decoration => MapObjectKind::DECORATION.to_string(),
        }
    }
}

#[cfg(not(feature = "ultimate"))]
impl ComboBoxValue for MapObjectKind {
    fn get_index(&self) -> usize {
        match self {
            Self::Item => 0,
            Self::Environment => 1,
            Self::Decoration => 2,
        }
    }

    fn set_index(&mut self, index: usize) {
        *self = match index {
            0 => Self::Item,
            1 => Self::Environment,
            2 => Self::Decoration,
            _ => unreachable!(),
        }
    }

    fn get_options(&self) -> Vec<String> {
        Self::options().iter().map(|s| s.to_string()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapObject {
    pub id: String,
    pub kind: MapObjectKind,
    #[serde(with = "core::json::vec2_def")]
    pub position: Vec2,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, MapProperty>,
}

impl MapObject {
    pub fn new(id: &str, kind: MapObjectKind, position: Vec2) -> Self {
        MapObject {
            id: id.to_string(),
            kind,
            position,
            properties: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapTileset {
    pub id: String,
    pub texture_id: String,
    pub texture_size: Size<u32>,
    pub tile_size: Size<f32>,
    pub grid_size: Size<u32>,
    pub first_tile_id: u32,
    pub tile_cnt: u32,
    #[serde(
        default = "MapTileset::default_tile_subdivisions",
        with = "core::json::uvec2_def"
    )]
    pub tile_subdivisions: UVec2,
    pub autotile_mask: Vec<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tile_attributes: HashMap<u32, Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, MapProperty>,
}

impl MapTileset {
    pub fn new(
        id: &str,
        texture_id: &str,
        texture_size: Size<u32>,
        tile_size: Size<f32>,
        first_tile_id: u32,
    ) -> Self {
        let grid_size = Size::new(
            texture_size.width / tile_size.width as u32,
            texture_size.height / tile_size.height as u32,
        );

        let tile_subdivisions = Self::default_tile_subdivisions();

        let subtile_grid_size: Size<u32> = grid_size * tile_subdivisions.into();

        let subtile_cnt = (subtile_grid_size.width * subtile_grid_size.height) as usize;

        let mut autotile_mask = vec![];
        autotile_mask.resize(subtile_cnt, false);

        MapTileset {
            id: id.to_string(),
            texture_id: texture_id.to_string(),
            texture_size,
            tile_size,
            grid_size,
            first_tile_id,
            tile_cnt: grid_size.width * grid_size.height,
            tile_subdivisions,
            autotile_mask,
            tile_attributes: HashMap::new(),
            properties: HashMap::new(),
        }
    }

    pub fn get_texture_coords(&self, tile_id: u32) -> Vec2 {
        let x = (tile_id % self.grid_size.width) as f32 * self.tile_size.width;
        let y = (tile_id / self.grid_size.width) as f32 * self.tile_size.height;
        vec2(x, y)
    }

    pub fn default_tile_subdivisions() -> UVec2 {
        uvec2(3, 3)
    }
}
