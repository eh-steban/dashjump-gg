//! Entity position extraction utilities

use haste::entities::{Entity, deadlock_coord_from_cell, fkey_from_path};

/// Get a single coordinate from an entity's cell and vector components.
/// Returns None if position fields are not yet populated (e.g. on CREATE before first UPDATE).
pub fn get_entity_coord(entity: &Entity, cell_key: &u64, vec_key: &u64) -> Option<f32> {
    let cell: u16 = entity.get_value(cell_key)?;
    let vec: f32 = entity.get_value(vec_key)?;
    Some(deadlock_coord_from_cell(cell, vec))
}

// In the deadlock-api/haste fork, the send_node for CBodyComponent sub-fields is just
// "CBodyComponent" (not the full "CBodyComponent.m_skeletonInstance.m_vecOrigin" path
// that haste-inspector displays). The stored key hash matches the 2-level path.
const CX: u64 = fkey_from_path(&["CBodyComponent", "m_cellX"]);
const CY: u64 = fkey_from_path(&["CBodyComponent", "m_cellY"]);
const CZ: u64 = fkey_from_path(&["CBodyComponent", "m_cellZ"]);
const VX: u64 = fkey_from_path(&["CBodyComponent", "m_vecX"]);
const VY: u64 = fkey_from_path(&["CBodyComponent", "m_vecY"]);
const VZ: u64 = fkey_from_path(&["CBodyComponent", "m_vecZ"]);

/// Get full [x, y, z] world position from an entity.
/// Returns None if position fields are not yet populated (e.g. on CREATE before first UPDATE).
pub fn get_entity_position(entity: &Entity) -> Option<[f32; 3]> {
    let x = get_entity_coord(entity, &CX, &VX)?;
    let y = get_entity_coord(entity, &CY, &VY)?;
    let z = get_entity_coord(entity, &CZ, &VZ)?;
    Some([x, y, z])
}
