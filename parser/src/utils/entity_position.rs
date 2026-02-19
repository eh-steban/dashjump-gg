//! Entity position extraction utilities

use haste::entities::{deadlock_coord_from_cell, fkey_from_path, Entity};

/// Get a single coordinate from an entity's cell and vector components
pub fn get_entity_coord(entity: &Entity, cell_key: &u64, vec_key: &u64) -> f32 {
    let cell: u16 = entity.get_value(cell_key).unwrap();
    let vec: f32 = entity.get_value(vec_key).unwrap();
    deadlock_coord_from_cell(cell, vec)
}

/// Get full [x, y, z] position from an entity.
pub fn get_entity_position(entity: &Entity) -> [f32; 3] {
    const CX: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]);
    const CY: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellY"]);
    const CZ: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellZ"]);

    const VX: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecX"]);
    const VY: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecY"]);
    const VZ: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecZ"]);

    let x = get_entity_coord(entity, &CX, &VX);
    let y = get_entity_coord(entity, &CY, &VY);
    let z = get_entity_coord(entity, &CZ, &VZ);

    [x, y, z]
}
