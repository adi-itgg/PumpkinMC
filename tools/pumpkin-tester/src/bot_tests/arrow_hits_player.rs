use std::{sync::Arc, time::Duration};

use pumpkin_core::{
    entity::{
        Entity, EntityBase,
        projectile::arrow::{ArrowEntity, ArrowPickup},
    },
    server::Server,
};
use pumpkin_data::{entity::EntityType, item::Item, item_stack::ItemStack};
use pumpkin_util::GameMode;

use crate::bot::{HeadlessBot, MAX_POLL_TICKS};

/// An arrow fired at a player must stop on them and deal damage.
///
/// Regression for arrows ignoring players: `World::get_entities_at_box` only walks
/// non-player entities, so the arrow collision raycast used to miss players entirely
/// and the arrow flew through them without damage.
pub async fn run_arrow_hits_player_test(
    server: &Arc<Server>,
    bot: &HeadlessBot,
    summary_lines: &mut Vec<String>,
) -> Result<(), String> {
    let player = server
        .get_player_by_name(&bot.name)
        .ok_or("Missing test bot player")?;
    let world = player.world();

    let original_mode = player.gamemode.load();
    // Creative and spectator players are invulnerable.
    player.set_gamemode(GameMode::Survival);
    let health_before = player.living_entity.health.load();

    let player_pos = player.position();
    let shooter_pos = player_pos.add_raw(0.0, 0.0, 10.0);
    let shooter = Entity::new(world.clone(), shooter_pos, &EntityType::SKELETON);
    let arrow = ArrowEntity::new_shot(
        Entity::new(world.clone(), shooter_pos, &EntityType::ARROW),
        &shooter,
        &ItemStack::new(1, &Item::ARROW),
        ArrowPickup::Disallowed,
    );
    arrow.set_velocity(
        player_pos.x - shooter_pos.x,
        player_pos.y - shooter_pos.y,
        player_pos.z - shooter_pos.z,
        1.6,
        0.0,
    );
    arrow.set_base_damage(5.0);

    let arrow_id = arrow.entity.entity_id;
    let spawned = world.spawn_entity(Arc::new(arrow) as Arc<dyn EntityBase>);
    let outcome = if spawned {
        let mut hit = false;
        for _ in 0..MAX_POLL_TICKS {
            tokio::time::sleep(Duration::from_millis(5)).await;
            server.tick();
            if player.living_entity.health.load() < health_before {
                hit = true;
                break;
            }
            // A stuck or removed arrow can no longer reach the player.
            if world.get_entity_by_id(arrow_id).is_none() {
                break;
            }
        }
        if hit {
            Ok(())
        } else {
            Err(format!(
                "Arrow flew through the player without damage (health stayed {health_before})"
            ))
        }
    } else {
        Err("Could not spawn the test arrow".into())
    };

    player.set_gamemode(original_mode);
    summary_lines.push("bot: arrow_deals_damage_to_player".to_string());
    outcome
}
