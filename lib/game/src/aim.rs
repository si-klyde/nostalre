//! Controller aim mode: a skill armed from the pad picks its target from the right stick instead of the
//! cursor. The enemy most on aim within a cone from the player is picked and held; melee skills take the
//! nearest one, Y the next.

use crate::entity::{Entity, EntityState, EntityType};
use crate::targeting::{
    MapProperties, RayHit, TargetClass, can_attack, nearest_cycle, ray_pick, skill_target_allowed,
};

/// Offensive skills reaching this many cells or fewer take the nearest enemy, not the ray.
pub const MELEE_RANGE: i32 = 2;

/// Map-space unit vectors for RO's eight directions (0 north, counter-clockwise).
const DIRECTIONS: [(f32, f32); 8] = {
    const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
    [
        (0.0, 1.0),
        (-D, D),
        (-1.0, 0.0),
        (-D, -D),
        (0.0, -1.0),
        (D, -D),
        (1.0, 0.0),
        (D, D),
    ]
};

/// The aim on the map: the stick (raw, y down) turned by the camera's `yaw`; at rest the `last` aim, else the
/// player's `facing`.
pub fn aim_dir(stick: (f32, f32), yaw: f32, last: Option<(f32, f32)>, facing: u8) -> (f32, f32) {
    let (sx, sy) = (stick.0, -stick.1);
    let len = (sx * sx + sy * sy).sqrt();
    if len <= f32::EPSILON {
        return last.unwrap_or(DIRECTIONS[usize::from(facing & 7)]);
    }
    // screen up is +y on the map at yaw 0; the camera's yaw turns the screen over the map, as `walk_target`
    let (sin, cos) = yaw.sin_cos();
    ((sx * cos - sy * sin) / len, (sx * sin + sy * cos) / len)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimTarget {
    Entity(u32),
    /// A buff's target: picked like an entity, shown with the lock cursor
    Ally(u32),
    Cell((i32, i32)),
    Nothing,
}

/// A target is picked within this many degrees of the stick.
const ACQUIRE_DEG: f32 = 20.0;
/// A held target stays locked within this many.
const KEEP_DEG: f32 = 40.0;
/// Another target takes the lock only when this many degrees more on aim.
const SWITCH_DEG: f32 = 15.0;
/// Targets this close in angle count as level: the nearer wins.
const LEVEL_DEG: f32 = 3.0;
/// Wall sampling step along the line of sight, in cells.
const SIGHT_STEP: f32 = 0.25;

pub struct AimInput<'a> {
    /// The player where it is drawn, in cells.
    pub origin: (f32, f32),
    pub dir: (f32, f32),
    pub range: i32,
    pub stick_tilted: bool,
    /// Y this frame: step to the next melee target.
    pub cycle: bool,
    /// The target aimed last frame.
    pub current: Option<u32>,
    /// Ids and drawn positions of the actors that pass [`aim_candidate`]: smooth, so a walking target does not
    /// jump a cell at a time.
    pub candidates: &'a [(u32, (f32, f32))],
}

/// What an armed skill of `class` aims at this frame.
pub fn aim_target(
    class: TargetClass,
    input: &AimInput,
    blocked: impl Fn(i32, i32) -> bool,
) -> AimTarget {
    match class {
        TargetClass::Ground => match cone_pick(input, &blocked) {
            Some((_, (x, y))) => AimTarget::Cell((x.round() as i32, y.round() as i32)),
            None => AimTarget::Cell(ray_end(input, &blocked)),
        },
        TargetClass::Offensive if input.range <= MELEE_RANGE && !input.stick_tilted => {
            melee_target(input)
        }
        TargetClass::Offensive => {
            cone_pick(input, &blocked).map_or(AimTarget::Nothing, |(id, _)| AimTarget::Entity(id))
        }
        _ => AimTarget::Nothing,
    }
}

/// The target the stick points at: the held one while it stays within [`KEEP_DEG`] unless another is
/// [`SWITCH_DEG`] more on aim, else the most on-aim within [`ACQUIRE_DEG`]. Only in range and in sight.
fn cone_pick(input: &AimInput, blocked: &impl Fn(i32, i32) -> bool) -> Option<(u32, (f32, f32))> {
    let len = (input.dir.0 * input.dir.0 + input.dir.1 * input.dir.1).sqrt();
    if len <= f32::EPSILON {
        return None;
    }
    let dir = (input.dir.0 / len, input.dir.1 / len);
    let mut held: Option<(u32, (f32, f32), f32)> = None;
    let mut best: Option<(u32, (f32, f32), f32, f32)> = None;
    for &(id, pos) in input.candidates {
        let (rx, ry) = (pos.0 - input.origin.0, pos.1 - input.origin.1);
        let dist = (rx * rx + ry * ry).sqrt();
        if dist < 0.5 || rx.abs().max(ry.abs()) > input.range as f32 + 0.5 {
            continue;
        }
        let angle = ((rx * dir.0 + ry * dir.1) / dist)
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        if angle > KEEP_DEG || !in_sight(input.origin, pos, blocked) {
            continue;
        }
        if Some(id) == input.current {
            held = Some((id, pos, angle));
        }
        let better = best.is_none_or(|(_, _, a, d)| {
            angle < a - LEVEL_DEG || (angle <= a + LEVEL_DEG && dist < d)
        });
        if angle <= ACQUIRE_DEG && better {
            best = Some((id, pos, angle, dist));
        }
    }
    match (held, best) {
        (Some((_, _, held_angle)), Some((id, pos, angle, _)))
            if angle <= held_angle - SWITCH_DEG =>
        {
            Some((id, pos))
        }
        (Some((id, pos, _)), _) => Some((id, pos)),
        (None, best) => best.map(|(id, pos, ..)| (id, pos)),
    }
}

/// No blocked cell on the straight line between `from` and `to` (both ends left out).
fn in_sight(from: (f32, f32), to: (f32, f32), blocked: &impl Fn(i32, i32) -> bool) -> bool {
    let cell = |p: (f32, f32)| (p.0.round() as i32, p.1.round() as i32);
    let (start, end) = (cell(from), cell(to));
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = ((dx * dx + dy * dy).sqrt() / SIGHT_STEP).ceil() as i32;
    (1..steps).all(|i| {
        let t = i as f32 / steps as f32;
        let c = cell((from.0 + dx * t, from.1 + dy * t));
        c == start || c == end || !blocked(c.0, c.1)
    })
}

/// Where a ground skill lands with no target: the ray's last open cell within range.
fn ray_end(input: &AimInput, blocked: &impl Fn(i32, i32) -> bool) -> (i32, i32) {
    let origin = (input.origin.0.round() as u16, input.origin.1.round() as u16);
    match ray_pick(origin, input.dir, input.range, std::iter::empty(), blocked) {
        RayHit::Cell(cell) => cell,
        RayHit::Entity { cell, .. } => (i32::from(cell.0), i32::from(cell.1)),
    }
}

/// The held target while in reach, else the nearest; on Y the next by distance.
fn melee_target(input: &AimInput) -> AimTarget {
    let (ox, oy) = (input.origin.0.round() as i32, input.origin.1.round() as i32);
    let in_reach: Vec<(u32, i32)> = input
        .candidates
        .iter()
        .map(|&(id, (x, y))| {
            let (x, y) = (x.round() as i32, y.round() as i32);
            (id, (x - ox).abs().max((y - oy).abs()))
        })
        .filter(|&(_, dist)| dist <= input.range)
        .collect();
    let held = input
        .current
        .filter(|id| !input.cycle && in_reach.iter().any(|c| c.0 == *id));
    held.or_else(|| nearest_cycle(input.current, &in_reach))
        .map_or(AimTarget::Nothing, AimTarget::Entity)
}

/// Whether an actor can be aimed at by an armed skill of `class`.
pub fn aim_candidate(
    target: &Entity,
    class: TargetClass,
    map: &MapProperties,
    player_id: Option<u32>,
) -> bool {
    if !target.is_alive() {
        return false;
    }
    match class {
        TargetClass::Ground => can_attack(target, map, player_id),
        _ => skill_target_allowed(class, target, map, player_id),
    }
}

/// The buff target: the `lock` while it is still offered, else the first offered, the caster (`self_id`,
/// none for a revive) ahead of the `allies` (id, distance) by distance. On Y the next of them, wrapping.
pub fn buff_target(
    self_id: Option<u32>,
    lock: Option<u32>,
    cycle: bool,
    allies: &[(u32, i32)],
) -> Option<u32> {
    let offered: Vec<(u32, i32)> = self_id
        .map(|id| (id, 0))
        .into_iter()
        .chain(allies.iter().copied())
        .collect();
    let held = lock.filter(|id| offered.iter().any(|c| c.0 == *id));
    if cycle {
        nearest_cycle(held, &offered)
    } else {
        held.or_else(|| nearest_cycle(None, &offered))
    }
}

/// Whether a player is offered to a buff aimed from the pad: in the caster's party, within `range` cells, and
/// dead for a skill that `revives` a corpse, alive for any other.
pub fn buff_candidate(
    target: &Entity,
    in_party: bool,
    origin: (u16, u16),
    range: i32,
    revives: bool,
) -> bool {
    let (x, y) = target.movement.cell_position();
    let dist = (i32::from(x) - i32::from(origin.0))
        .abs()
        .max((i32::from(y) - i32::from(origin.1)).abs());
    target.entity_type == EntityType::Player
        && in_party
        && !target.is_fading()
        && dist <= range
        && (target.state() == EntityState::Dead) == revives
}
