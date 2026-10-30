use nostalre_game::targeting::{RayHit, nearest_cycle, ray_pick};

const ORIGIN: (u16, u16) = (50, 50);
const UP: (f32, f32) = (0.0, 1.0);

fn open(_: i32, _: i32) -> bool {
    false
}

fn hit_id(hit: RayHit) -> Option<u32> {
    match hit {
        RayHit::Entity { id, .. } => Some(id),
        RayHit::Cell(_) => None,
    }
}

#[test]
fn ray_hits_first_enemy_in_line() {
    let enemies = [(2, (50, 55)), (1, (50, 53))];
    let hit = ray_pick(ORIGIN, UP, 9, enemies, open);
    assert_eq!(
        hit,
        RayHit::Entity {
            id: 1,
            cell: (50, 53)
        }
    );
}

#[test]
fn ray_skips_enemy_beside_the_line() {
    assert_eq!(hit_id(ray_pick(ORIGIN, UP, 9, [(1, (52, 54))], open)), None);
    // half a cell off the line: the ray leans so (51, 54) sits ~0.5 from it
    let leaning = (0.124, 0.992);
    assert_eq!(
        hit_id(ray_pick(ORIGIN, leaning, 9, [(1, (51, 54))], open)),
        Some(1)
    );
}

#[test]
fn ray_ignores_enemy_behind() {
    let enemies = [(1, (50, 47)), (2, ORIGIN)];
    assert_eq!(hit_id(ray_pick(ORIGIN, UP, 9, enemies, open)), None);
}

#[test]
fn ray_stops_at_wall() {
    let wall = |x: i32, y: i32| x == 50 && y == 52;
    let hit = ray_pick(ORIGIN, UP, 9, [(1, (50, 54))], wall);
    assert_eq!(hit, RayHit::Cell((50, 51)));
}

#[test]
fn ray_miss_returns_end_cell() {
    let none: [(u32, (u16, u16)); 0] = [];
    assert_eq!(ray_pick(ORIGIN, UP, 9, none, open), RayHit::Cell((50, 59)));
}

#[test]
fn ray_end_clamps_to_range() {
    let hit = ray_pick(ORIGIN, UP, 9, [(1, (50, 62))], open);
    assert_eq!(hit, RayHit::Cell((50, 59)));
}

#[test]
fn ray_tie_prefers_closer_to_line() {
    // same row: (50,54) is ~3.95 along and ~0.64 off, (51,54) ~4.11 along and ~0.35 off.
    // Within half a cell along the ray counts as level, so the one nearer the aim wins.
    let dir = (0.16, 0.987);
    let enemies = [(1, (50, 54)), (2, (51, 54))];
    assert_eq!(hit_id(ray_pick(ORIGIN, dir, 9, enemies, open)), Some(2));
}

#[test]
fn ray_hits_on_diagonal() {
    let diag = (
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    );
    assert_eq!(
        hit_id(ray_pick(ORIGIN, diag, 5, [(7, (53, 53))], open)),
        Some(7)
    );
}

#[test]
fn nearest_cycle_orders_and_wraps() {
    let candidates = [(30, 5), (10, 1), (20, 3)];
    assert_eq!(nearest_cycle(None, &candidates), Some(10));
    assert_eq!(nearest_cycle(Some(10), &candidates), Some(20));
    assert_eq!(nearest_cycle(Some(20), &candidates), Some(30));
    assert_eq!(nearest_cycle(Some(30), &candidates), Some(10));
    // a current target that left the list restarts at the nearest
    assert_eq!(nearest_cycle(Some(99), &candidates), Some(10));
    assert_eq!(nearest_cycle(None, &[]), None);
}
