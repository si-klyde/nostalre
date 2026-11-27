use crate::App;
use crate::game_state::{HoverState, attackable_unit_at};
use nostalre_game::aim::AimTarget;
use nostalre_game::app_state::AppState;
use nostalre_game::chat_room::ChatRoomRegistry;
use nostalre_game::cursor::{
    CompanionSkillTarget, CursorType, PendingSkillTarget, RenderEntry, cursor_type_for_cell,
    hovered_entity_cursor_type,
};
use nostalre_game::targeting::{TargetClass, skill_target_class};

/// The chat room whose board is under `mouse`. `owners` must hold every drawn actor, hidden ones too: a script
/// hides the NPC that hosts a waiting room (the Knight, Crusader and Sage job quests) and only its board shows.
fn chat_room_at(
    mouse: (f32, f32),
    rooms: &ChatRoomRegistry,
    owners: &[RenderEntry],
) -> Option<u32> {
    let (mx, my) = mouse;
    rooms.iter().find_map(|room| {
        let entry = owners.iter().find(|e| e.id == room.owner_aid)?;
        let r = crate::overlay::chat_room_board_rect(entry);
        (mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3]).then_some(room.room_id)
    })
}

impl App {
    pub(crate) fn update_grid_hover(&mut self) -> Option<(i32, i32)> {
        let hovered = if self.game.session.app_state == AppState::InGame {
            self.hovered_cell()
        } else {
            None
        };

        let hover_corners = hovered.and_then(|(cx, cy)| {
            let coords = self.game.session.map_coords.as_ref()?;
            let gat = self.game.session.gat.as_ref()?;
            Some(coords.cell_corners_world(gat, cx, cy))
        });

        let marker_corners: [Option<[[f32; 3]; 4]>; 2] = std::array::from_fn(|i| {
            let (cx, cy) = self.game.companions.companion_move_marker[i]?;
            let coords = self.game.session.map_coords.as_ref()?;
            let gat = self.game.session.gat.as_ref()?;
            Some(coords.cell_corners_world(gat, cx, cy))
        });

        if let Some(renderer) = &mut self.renderer
            && let Some(grid) = &mut renderer.grid_selector
        {
            if let Some(corners) = hover_corners {
                grid.update_hover(&renderer.device.queue, corners);
                grid.set_hover_visible(true);
            } else {
                grid.set_hover_visible(false);
            }
            for (i, corners) in marker_corners.iter().enumerate() {
                match corners {
                    Some(corners) => {
                        grid.update_marker(&renderer.device.queue, i, *corners);
                        grid.set_marker_visible(i, true);
                    }
                    None => grid.set_marker_visible(i, false),
                }
            }
        }

        hovered
    }

    fn hovered_vending_board(&self, render_list: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        let (mx, my) = (mx as f32, my as f32);
        for entry in render_list {
            let is_vendor = self
                .game
                .world
                .entities
                .get(entry.id)
                .is_some_and(|e| e.vending_board.is_some());
            if !is_vendor {
                continue;
            }
            let r = crate::overlay::vending_board_rect(entry);
            if mx >= r[0] && mx <= r[2] && my >= r[1] && my <= r[3] {
                return Some(entry.id);
            }
        }
        None
    }

    /// A deployed trap occupies exactly one cell, so the hovered cell picks it.
    fn hovered_trap(&self, hovered_cell: Option<(i32, i32)>) -> Option<u32> {
        let (cx, cy) = hovered_cell?;
        self.game
            .world
            .trap_units
            .iter()
            .find(|(_, trap)| trap.cell == (cx as i16, cy as i16))
            .map(|(&aid, _)| aid)
    }

    fn hovered_chat_room(&self, room_owners: &[RenderEntry]) -> Option<u32> {
        let (mx, my) = self.input.mouse_position;
        chat_room_at((mx as f32, my as f32), &self.game.chat_rooms, room_owners)
    }

    pub(crate) fn resolve_hover(
        &self,
        hovered_cell: Option<(i32, i32)>,
        render_list: &[RenderEntry],
        room_owners: &[RenderEntry],
        floor_item_render_list: &[RenderEntry],
        ui_any_hovered: bool,
        ui_any_interactive_hovered: bool,
    ) -> HoverState {
        let mut hover = HoverState::default();
        let mouse = self.input.mouse_position;
        let entities = &self.game.world.entities;
        let map = &self.game.session.map_properties;

        hover.hovered_player_id =
            nostalre_game::cursor::hovered_player(mouse, entities, render_list);
        hover.hovered_self_id = nostalre_game::cursor::hovered_self(mouse, entities, render_list);

        if let Some(gat) = &self.game.session.gat {
            hover.cell_cursor = cursor_type_for_cell(gat, hovered_cell);
        }

        // An armed companion target does not suppress picking: confirming the order
        // needs the same target to resolve under the cursor a second time.
        let suppressed =
            self.input.right_mouse_down || ui_any_interactive_hovered || ui_any_hovered;

        if !suppressed {
            if let Some(pending) = &self.game.pending_casts.pending_companion_skill {
                match pending.target {
                    CompanionSkillTarget::Entity => {
                        hover.hovered_entity_id =
                            hovered_entity_cursor_type(mouse, entities, render_list, map, None)
                                .map(|(_, id)| id);
                    }
                    CompanionSkillTarget::SkillUnit => {
                        hover.hovered_skill_unit_id = self.hovered_trap(hovered_cell);
                    }
                    CompanionSkillTarget::Ground => {}
                }
            } else if self.game.companions.capture_targeting {
                hover.hovered_entity_id = hovered_entity_cursor_type(
                    mouse,
                    entities,
                    render_list,
                    map,
                    Some(TargetClass::Offensive),
                )
                .map(|(_, id)| id);
            } else if let Some(PendingSkillTarget::SkillUnit { .. }) =
                &self.game.pending_casts.pending_skill_target
            {
                hover.hovered_skill_unit_id = self.hovered_trap(hovered_cell);
            } else if let Some(PendingSkillTarget::Entity { skill, .. }) =
                &self.game.pending_casts.pending_skill_target
            {
                let class = self
                    .game
                    .resolve_cast_skill(*skill)
                    .map(|(target_type, _)| skill_target_class(target_type))
                    .unwrap_or(TargetClass::Offensive);
                hover.hovered_entity_id =
                    hovered_entity_cursor_type(mouse, entities, render_list, map, Some(class))
                        .map(|(_, id)| id);
            } else if self.game.pending_casts.pending_skill_target.is_none() {
                if let Some(room_id) = self.hovered_chat_room(room_owners) {
                    hover.hovered_chat_room = Some(room_id);
                } else if let Some(vendor_id) = self.hovered_vending_board(render_list) {
                    hover.hovered_vending = Some(vendor_id);
                } else if let Some((cursor, id)) =
                    hovered_entity_cursor_type(mouse, entities, render_list, map, None)
                {
                    hover.hovered_entity_id = Some(id);
                    hover.hovered_entity_cursor = Some(cursor);
                } else if let Some(cell) = hovered_cell
                    && let Some(unit_id) = attackable_unit_at(&self.game.world.trap_units, cell)
                {
                    hover.hovered_skill_unit_id = Some(unit_id);
                    hover.hovered_entity_cursor = Some(CursorType::Attack);
                }
            }
        }

        if hover.target_id().is_none() && !ui_any_hovered && !self.input.right_mouse_down {
            let (mx, my) = (mouse.0 as f32, mouse.1 as f32);
            hover.hovered_floor_item_id = floor_item_render_list
                .iter()
                .find(|entry| {
                    mx >= entry.pick_bounds[0]
                        && mx <= entry.pick_bounds[2]
                        && my >= entry.pick_bounds[1]
                        && my <= entry.pick_bounds[3]
                })
                .map(|entry| entry.id);
        }

        // Pad aim mode: the aimed target stands in for whatever the frozen cursor is over
        if let Some(aim) = self.gamepad.aim_target() {
            hover = HoverState {
                hovered_player_id: hover.hovered_player_id,
                hovered_self_id: hover.hovered_self_id,
                cell_cursor: hover.cell_cursor,
                ..HoverState::default()
            };
            let aimed = match aim {
                AimTarget::Entity(id) => Some((id, CursorType::Attack)),
                AimTarget::Ally(id) => Some((id, CursorType::Lock)),
                AimTarget::Cell(_) | AimTarget::Nothing => None,
            };
            if let Some((id, cursor)) = aimed {
                hover.hovered_entity_id = Some(id);
                hover.hovered_entity_cursor = Some(cursor);
            }
        }

        hover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostalre_game::chat_room::ChatRoom;
    use nostalre_game::cursor::RenderEntryKind;

    fn owner(id: u32) -> RenderEntry {
        RenderEntry {
            kind: RenderEntryKind::Entity,
            id,
            screen_anchor: [400.0, 300.0],
            depth: 0.5,
            depth_gradient: [0.0, 0.0],
            flat_depth_gradient: [0.0, 0.0],
            camera_dir: 0,
            sprite_scale: 1.0,
            pick_bounds: [350.0, 200.0, 450.0, 300.0],
            head_offset: 100.0,
        }
    }

    fn rooms() -> ChatRoomRegistry {
        let mut rooms = ChatRoomRegistry::new();
        rooms.upsert(ChatRoom {
            room_id: 7,
            owner_aid: 110,
            title: "Waiting Room".into(),
            cur_count: 0,
            max_count: 20,
            atype: 1,
        });
        rooms
    }

    fn board_center(entry: &RenderEntry) -> (f32, f32) {
        let r = crate::overlay::chat_room_board_rect(entry);
        ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0)
    }

    #[test]
    fn board_of_a_drawn_owner_picks_its_room() {
        let owners = [owner(110)];
        assert_eq!(
            chat_room_at(board_center(&owners[0]), &rooms(), &owners),
            Some(7)
        );
    }

    #[test]
    fn cursor_off_the_board_picks_nothing() {
        assert_eq!(chat_room_at((0.0, 0.0), &rooms(), &[owner(110)]), None);
    }

    #[test]
    fn room_without_its_owner_in_the_list_picks_nothing() {
        let other = owner(999);
        assert_eq!(chat_room_at(board_center(&other), &rooms(), &[other]), None);
    }
}
