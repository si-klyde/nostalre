use models::enums::skill_enums::SkillEnum;
use nostalre_game::aim::{
    AimInput, AimTarget, MELEE_RANGE, aim_candidate, aim_dir, aim_target, buff_candidate,
    buff_target,
};
use nostalre_game::entity::{Entity, EntityState, EntityType};
use nostalre_game::movement::MovementState;
use nostalre_game::targeting::{MapKind, MapProperties, TargetClass, revives_corpse};

const ORIGIN: (f32, f32) = (50.0, 50.0);
const UP: (f32, f32) = (0.0, 1.0);

fn open(_: i32, _: i32) -> bool {
    false
}

fn close(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4
}

fn input<'a>(dir: (f32, f32), range: i32, candidates: &'a [(u32, (f32, f32))]) -> AimInput<'a> {
    AimInput {
        origin: ORIGIN,
        dir,
        range,
        stick_tilted: false,
        cycle: false,
        current: None,
        candidates,
    }
}

#[test]
fn aim_dir_turns_with_camera_yaw() {
    // stick up (y down on the pad) at yaw 0 is +y on the map, as `walk_target` reads it
    assert!(close(aim_dir((0.0, -1.0), 0.0, None, 0), (0.0, 1.0)));
    assert!(close(aim_dir((1.0, 0.0), 0.0, None, 0), (1.0, 0.0)));
    let quarter = std::f32::consts::FRAC_PI_2;
    assert!(close(aim_dir((0.0, -1.0), quarter, None, 0), (-1.0, 0.0)));
    // a partial tilt still gives a unit direction
    assert!(close(aim_dir((0.0, -0.3), 0.0, None, 0), (0.0, 1.0)));
}

#[test]
fn aim_dir_keeps_last_when_stick_at_rest() {
    assert!(close(
        aim_dir((0.0, 0.0), 0.0, Some((1.0, 0.0)), 0),
        (1.0, 0.0)
    ));
    // no last: the player's facing, RO directions (0 north, 2 west, 4 south, 6 east)
    assert!(close(aim_dir((0.0, 0.0), 0.0, None, 0), (0.0, 1.0)));
    assert!(close(aim_dir((0.0, 0.0), 0.0, None, 2), (-1.0, 0.0)));
    assert!(close(aim_dir((0.0, 0.0), 0.0, None, 4), (0.0, -1.0)));
    assert!(close(aim_dir((0.0, 0.0), 0.0, None, 6), (1.0, 0.0)));
    let d = std::f32::consts::FRAC_1_SQRT_2;
    assert!(close(aim_dir((0.0, 0.0), 0.0, None, 7), (d, d)));
}

#[test]
fn aim_target_ranged_uses_cone() {
    let mobs = [(1, (50.0, 55.0)), (2, (47.0, 50.0))];
    let aimed = aim_target(TargetClass::Offensive, &input(UP, 9, &mobs), open);
    assert_eq!(aimed, AimTarget::Entity(1));
    let aimed = aim_target(TargetClass::Offensive, &input((1.0, 0.0), 9, &mobs), open);
    assert_eq!(aimed, AimTarget::Nothing);
}

#[test]
fn aim_target_ground_hit_or_end() {
    let mobs = [(1, (50.0, 55.0))];
    let aimed = aim_target(TargetClass::Ground, &input(UP, 9, &mobs), open);
    assert_eq!(aimed, AimTarget::Cell((50, 55)));
    let aimed = aim_target(TargetClass::Ground, &input((1.0, 0.0), 9, &mobs), open);
    assert_eq!(aimed, AimTarget::Cell((59, 50)));
}

#[test]
fn aim_target_melee_nearest_then_cycle() {
    // nothing on the faced line, two in reach behind and beside
    let mobs = [(1, (52.0, 48.0)), (2, (49.0, 49.0)), (3, (58.0, 58.0))];
    let mut aim = input(UP, MELEE_RANGE, &mobs);
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, open),
        AimTarget::Entity(2)
    );
    // a held target stays while it is in reach
    aim.current = Some(1);
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, open),
        AimTarget::Entity(1)
    );
    // Y: the next by distance, wrapping; out of reach (3) never joins
    aim.cycle = true;
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, open),
        AimTarget::Entity(2)
    );
    aim.current = Some(2);
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, open),
        AimTarget::Entity(1)
    );
    let none: [(u32, (f32, f32)); 0] = [];
    assert_eq!(
        aim_target(TargetClass::Offensive, &input(UP, MELEE_RANGE, &none), open),
        AimTarget::Nothing
    );
}

#[test]
fn aim_target_melee_stick_uses_cone() {
    // (49,49) is nearest but behind; the tilted stick aims up at (50,52)
    let mobs = [(1, (50.0, 52.0)), (2, (49.0, 49.0))];
    let mut aim = input(UP, MELEE_RANGE, &mobs);
    aim.stick_tilted = true;
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, open),
        AimTarget::Entity(1)
    );
}

fn aimed(dir: (f32, f32), current: Option<u32>, mobs: &[(u32, (f32, f32))]) -> AimTarget {
    let mut aim = input(dir, 9, mobs);
    aim.current = current;
    aim_target(TargetClass::Offensive, &aim, open)
}

#[test]
fn cone_catches_an_off_line_target() {
    // ~14 degrees off the stick: caught, though 2 cells beside the line
    assert_eq!(aimed(UP, None, &[(1, (52.0, 58.0))]), AimTarget::Entity(1));
    // ~34 degrees off: outside the 20-degree cone
    assert_eq!(aimed(UP, None, &[(1, (54.0, 56.0))]), AimTarget::Nothing);
}

#[test]
fn cone_prefers_the_most_on_aim() {
    let mobs = [(1, (51.0, 55.0)), (2, (50.0, 58.0))];
    assert_eq!(aimed(UP, None, &mobs), AimTarget::Entity(2));
}

#[test]
fn lock_holds_inside_the_wider_keep_cone() {
    // locked at ~34 degrees: too far off to pick, close enough to keep
    assert_eq!(
        aimed(UP, Some(1), &[(1, (54.0, 56.0))]),
        AimTarget::Entity(1)
    );
    // a mob slightly more on-aim (~11 degrees better) does not steal the lock
    let mobs = [(1, (51.0, 55.0)), (2, (50.0, 58.0))];
    assert_eq!(aimed(UP, Some(1), &mobs), AimTarget::Entity(1));
}

#[test]
fn lock_switches_when_another_is_clearly_on_aim() {
    let mobs = [(1, (54.0, 56.0)), (2, (50.0, 57.0))];
    assert_eq!(aimed(UP, Some(1), &mobs), AimTarget::Entity(2));
}

#[test]
fn lock_drops_past_the_keep_cone_range_or_a_wall() {
    // ~72 degrees off
    assert_eq!(aimed(UP, Some(1), &[(1, (56.0, 52.0))]), AimTarget::Nothing);
    // out of range 9
    assert_eq!(aimed(UP, Some(1), &[(1, (50.0, 62.0))]), AimTarget::Nothing);
    let wall = |x: i32, y: i32| x == 50 && y == 52;
    let mut aim = input(UP, 9, &[(1, (50.0, 55.0))]);
    aim.current = Some(1);
    assert_eq!(
        aim_target(TargetClass::Offensive, &aim, wall),
        AimTarget::Nothing
    );
}

#[test]
fn a_walking_target_stays_locked_between_cells() {
    // drawn mid-step: the lock follows the smooth position
    for y in [55.0, 55.4, 55.8, 56.2] {
        assert_eq!(aimed(UP, Some(1), &[(1, (50.6, y))]), AimTarget::Entity(1));
    }
}

#[test]
fn ground_lands_under_a_cone_target() {
    let aim = input(UP, 9, &[(1, (52.0, 58.0))]);
    assert_eq!(
        aim_target(TargetClass::Ground, &aim, open),
        AimTarget::Cell((52, 58))
    );
}

fn entity(id: u32, entity_type: EntityType, job: u16) -> Entity {
    Entity::new(
        id,
        entity_type,
        job,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        100,
        100,
        0,
        150,
    )
}

#[test]
fn aim_candidates_filter() {
    let me = Some(1u32);
    let town = MapProperties::from_kind(MapKind::Normal);
    let mob = entity(10, EntityType::Monster, 1002);
    let mut pet = entity(11, EntityType::Monster, 1002);
    pet.is_pet = true;
    let mut dead = entity(12, EntityType::Monster, 1002);
    dead.set_state(EntityState::Dead);
    let npc = entity(13, EntityType::Npc, 60);
    let homun = entity(14, EntityType::Homunculus, 6001);
    let myself = entity(1, EntityType::Player, 0);
    let other = entity(20, EntityType::Player, 0);

    for class in [TargetClass::Offensive, TargetClass::Ground] {
        assert!(aim_candidate(&mob, class, &town, me), "{class:?}");
        for e in [&pet, &dead, &npc, &homun, &myself, &other] {
            assert!(!aim_candidate(e, class, &town, me), "{class:?} {}", e.id);
        }
    }
    let pvp = MapProperties::from_kind(MapKind::FreePvp);
    assert!(aim_candidate(&other, TargetClass::Offensive, &pvp, me));
    assert!(!aim_candidate(&myself, TargetClass::Offensive, &pvp, me));
}

const ME: u32 = 1;

#[test]
fn buff_target_defaults_to_self() {
    assert_eq!(buff_target(Some(ME), None, false, &[]), Some(ME));
    assert_eq!(buff_target(Some(ME), None, false, &[(20, 3)]), Some(ME));
}

#[test]
fn buff_cycle_self_then_party_by_distance() {
    let allies = [(30, 6), (20, 3)];
    assert_eq!(buff_target(Some(ME), Some(ME), true, &allies), Some(20));
    assert_eq!(buff_target(Some(ME), Some(20), true, &allies), Some(30));
    assert_eq!(buff_target(Some(ME), Some(30), true, &allies), Some(ME));
    // no party: Y stays on self
    assert_eq!(buff_target(Some(ME), Some(ME), true, &[]), Some(ME));
}

#[test]
fn buff_lock_persists() {
    assert_eq!(
        buff_target(Some(ME), Some(30), false, &[(20, 3), (30, 6)]),
        Some(30)
    );
}

#[test]
fn buff_lock_falls_back_to_self() {
    // 30 left reach (or the screen): no longer offered
    assert_eq!(buff_target(Some(ME), Some(30), false, &[(20, 3)]), Some(ME));
}

#[test]
fn buff_candidates_filter() {
    let origin = (50, 50);
    let mut near = entity(20, EntityType::Player, 0);
    near.movement = MovementState::new(53, 50);
    let mut far = entity(21, EntityType::Player, 0);
    far.movement = MovementState::new(60, 50);
    let mob = entity(10, EntityType::Monster, 1002);

    assert!(buff_candidate(&near, true, origin, 9, false));
    assert!(
        !buff_candidate(&near, false, origin, 9, false),
        "not in the party"
    );
    assert!(
        !buff_candidate(&far, true, origin, 9, false),
        "out of reach"
    );
    assert!(!buff_candidate(&mob, true, origin, 9, false), "a monster");
}

#[test]
fn buff_candidates_skip_dead_unless_revive() {
    let origin = (50, 50);
    let mut ally = entity(20, EntityType::Player, 0);
    ally.movement = MovementState::new(52, 50);
    let mut corpse = entity(21, EntityType::Player, 0);
    corpse.movement = MovementState::new(52, 50);
    corpse.set_state(EntityState::Dead);

    assert!(buff_candidate(&ally, true, origin, 9, false));
    assert!(
        !buff_candidate(&corpse, true, origin, 9, false),
        "a buff skips the dead"
    );
    assert!(
        buff_candidate(&corpse, true, origin, 9, true),
        "Resurrection takes the dead"
    );
    assert!(
        !buff_candidate(&ally, true, origin, 9, true),
        "Resurrection skips the living"
    );

    assert!(revives_corpse(SkillEnum::AllResurrection));
    assert!(!revives_corpse(SkillEnum::AlHeal));
}

#[test]
fn resurrection_starts_on_the_nearest_corpse() {
    // a revive leaves the caster out: the nearest corpse first, none without one
    let corpses = [(30, 6), (20, 3)];
    assert_eq!(buff_target(None, None, false, &corpses), Some(20));
    assert_eq!(buff_target(None, Some(20), true, &corpses), Some(30));
    assert_eq!(buff_target(None, Some(30), true, &corpses), Some(20));
    assert_eq!(buff_target(None, Some(30), false, &corpses), Some(30));
    assert_eq!(buff_target(None, None, false, &[]), None);
}
