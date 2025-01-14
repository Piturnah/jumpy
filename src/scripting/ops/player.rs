use crate::{
    player::{
        PlayerDespawnCommand, PlayerKillCommand, PlayerSetInventoryCommand, PlayerUseItemCommand,
    },
    prelude::*,
};
use anyhow::Context;
use bevy::ecs::system::Command;
use bevy_mod_js_scripting::{serde_json, JsRuntimeOp, JsValueRef, JsValueRefs, OpContext};

pub struct PlayerKill;
impl JsRuntimeOp for PlayerKill {
    fn js(&self) -> Option<&'static str> {
        Some(
            r#"
            if (!globalThis.Player) {
                globalThis.Player = {}
            }
            
            globalThis.Player.kill = (entity) => {
                return bevyModJsScriptingOpSync('jumpy_player_kill', Value.unwrapValueRef(entity));
            }
            "#,
        )
    }

    fn run(
        &self,
        ctx: OpContext,
        world: &mut World,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (value_ref,): (JsValueRef,) = serde_json::from_value(args).context("Parse args")?;
        let value_refs = ctx.op_state.get_mut::<JsValueRefs>().unwrap();
        let entity = value_ref.get_entity(world, value_refs)?;

        PlayerKillCommand::new(entity).write(world);

        Ok(serde_json::Value::Null)
    }
}

pub struct PlayerDespawn;
impl JsRuntimeOp for PlayerDespawn {
    fn js(&self) -> Option<&'static str> {
        Some(
            r#"
            if (!globalThis.Player) {
                globalThis.Player = {}
            }
            
            globalThis.Player.despawn = (entity) => {
                return bevyModJsScriptingOpSync('jumpy_player_despawn', Value.unwrapValueRef(entity));
            }
            "#,
        )
    }

    fn run(
        &self,
        ctx: OpContext,
        world: &mut World,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (value_ref,): (JsValueRef,) = serde_json::from_value(args).context("Parse args")?;
        let value_refs = ctx.op_state.get_mut::<JsValueRefs>().unwrap();
        let entity = value_ref.get_entity(world, value_refs)?;

        PlayerDespawnCommand::new(entity).write(world);

        Ok(serde_json::Value::Null)
    }
}

pub struct PlayerGetInventory;
impl JsRuntimeOp for PlayerGetInventory {
    fn js(&self) -> Option<&'static str> {
        Some(
            r#"
            if (!globalThis.Player) {
                globalThis.Player = {}
            }
            
            globalThis.Player.getInventory = (entity) => {
                return Value.wrapValueRef(bevyModJsScriptingOpSync(
                    'jumpy_player_get_inventory',
                    Value.unwrapValueRef(entity)
                ));
            }
            "#,
        )
    }

    fn run(
        &self,
        ctx: OpContext,
        world: &mut World,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (value_ref,): (JsValueRef,) = serde_json::from_value(args).context("Parse args")?;
        let value_refs = ctx.op_state.get_mut::<JsValueRefs>().unwrap();
        let player_ent = value_ref.get_entity(world, value_refs)?;

        let item_ent = crate::player::get_player_inventory(world, player_ent);
        let inventory = item_ent.map(|x| JsValueRef::new_free(Box::new(x), value_refs));

        Ok(serde_json::to_value(&inventory)?)
    }
}

pub struct PlayerSetInventory;
impl JsRuntimeOp for PlayerSetInventory {
    fn js(&self) -> Option<&'static str> {
        Some(
            r#"
            if (!globalThis.Player) {
                globalThis.Player = {}
            }
            
            globalThis.Player.setInventory = (player, inventory) => {
                return bevyModJsScriptingOpSync(
                    'jumpy_player_set_inventory',
                    Value.unwrapValueRef(player),
                    Value.unwrapValueRef(inventory)
                );
            }
            "#,
        )
    }

    fn run(
        &self,
        ctx: OpContext,
        world: &mut World,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (player_ref, item_ref): (JsValueRef, Option<JsValueRef>) =
            serde_json::from_value(args).context("Parse args")?;
        let value_refs = ctx.op_state.get_mut::<JsValueRefs>().unwrap();
        let player_ent = player_ref.get_entity(world, value_refs)?;
        let item_ent = item_ref
            .map(|x| x.get_entity(world, value_refs))
            .transpose()?;

        PlayerSetInventoryCommand::new(player_ent, item_ent).write(world);

        Ok(serde_json::Value::Null)
    }
}

pub struct PlayerUseItem;
impl JsRuntimeOp for PlayerUseItem {
    fn js(&self) -> Option<&'static str> {
        Some(
            r#"
            if (!globalThis.Player) {
                globalThis.Player = {}
            }
            
            globalThis.Player.useItem = (player) => {
                return bevyModJsScriptingOpSync(
                    'jumpy_player_use_item',
                    Value.unwrapValueRef(player)
                );
            }
            "#,
        )
    }

    fn run(
        &self,
        ctx: OpContext,
        world: &mut World,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let (player_ref,): (JsValueRef,) = serde_json::from_value(args).context("Parse args")?;
        let value_refs = ctx.op_state.get_mut::<JsValueRefs>().unwrap();
        let player_ent = player_ref.get_entity(world, value_refs)?;

        PlayerUseItemCommand::new(player_ent).write(world);

        Ok(serde_json::Value::Null)
    }
}
