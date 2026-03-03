//! Entity position extraction utilities

use haste::entities::{Entity, deadlock_coord_from_cell, fkey_from_path};

/// Get a single coordinate from an entity's cell and vector components.
pub fn get_entity_coord(entity: &Entity, cell_key: &u64, vec_key: &u64) -> f32 {
    let cell: u16 = entity.get_value(cell_key).unwrap();
    let vec: f32 = entity.get_value(vec_key).unwrap();
    deadlock_coord_from_cell(cell, vec)
}

const CX_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_cellX",
]);
const CY_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_cellY",
]);
const CZ_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_cellZ",
]);
const VX_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_vecX",
]);
const VY_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_vecY",
]);
const VZ_SKEL: u64 = fkey_from_path(&[
    "CBodyComponent",
    "m_skeletonInstance",
    "m_vecOrigin",
    "m_vecZ",
]);

/// Get full [x, y, z] world position from an entity.
pub fn get_entity_position(entity: &Entity) -> [f32; 3] {
    let x = get_entity_coord(entity, &CX_SKEL, &VX_SKEL);
    let y = get_entity_coord(entity, &CY_SKEL, &VY_SKEL);
    let z = get_entity_coord(entity, &CZ_SKEL, &VZ_SKEL);
    [x, y, z]
}
