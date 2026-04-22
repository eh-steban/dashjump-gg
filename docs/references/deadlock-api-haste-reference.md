# deadlock-api/haste Parser Reference
**Source:** deadlock-api/haste (fork of blukai/haste, branch: main)
**Last Fetched:** 2026-03-15
**Purpose:** Reference for dashJump parser agents implementing new message listeners

---

## Development Tools

### haste-inspector
**URL:** https://blukai.github.io/haste-inspector/
**What it is:** A browser-based SPA for inspecting Deadlock replay entity data. Load a `.dem` file via the browser UI and browse entity class names, field names, field types, and live values at any tick.

**Usage:** Drag-and-drop (or file-picker) a `.dem` file into the page. The inspector renders the full entity tree with field paths and types -- the same paths used by `fkey_from_path` in the parser.

**Primary use cases:**
- Confirming a field name before writing a `fkey_from_path` constant
- Verifying a field's type (`u64`, `f32`, `bool`, etc.) before calling `entity.get_value::<T>()`
- Checking whether a field is present on a given entity class
- Noticing when fields have been added or removed after a Deadlock patch

**Limitations for automated use:** The inspector is a JavaScript SPA with no API or data endpoint. `WebFetch` returns only the page title (no server-rendered content). Playwright can open the page but cannot browse entity fields without uploading a `.dem` file interactively. Treat this as a **human-in-the-loop tool** -- use it manually when investigating a field, not in agent workflows.

---

## What is haste?

A high-performance Rust replay parser for Deadlock and Dota 2 that processes `.dem` files into structured game events. A 31-minute Deadlock match parses in ~660ms with ~17.5MB peak memory.

---

## Parse Lifecycle

1. **Open file** -- `DemoFile::start_reading(BufReader::new(file))` reads and validates the demo header (`PBDEMS2\0` magic bytes). Returns a `DemoFile` which implements `DemoStream`.
2. **Create parser** -- `Parser::from_stream_with_visitor(demo_file, visitor)` wraps the stream and your visitor. No parsing happens yet.
3. **Run** -- `parser.run_to_end().await` enters the command loop.
4. **Command loop** -- Each iteration reads a `CmdHeader` (command type + tick + body size), then dispatches:
   - `DemSignonPacket` / `DemPacket` -- unpacked into a `CDemoPacket`, which contains a stream of sub-messages
   - `DemSendTables` -- parsed once into `FlattenedSerializerContainer` (schema for entity fields)
   - `DemClassInfo` -- parsed once into `EntityClasses` + populates `InstanceBaseline`
   - `DemFullPacket` -- snapshot used for seeking; contains string tables + packet
5. **Sub-message dispatch** -- Inside each packet, a bit-packed stream of `(command: u32, size: u32, data: &[u8])` tuples is read. The parser handles `SvcCreateStringTable`, `SvcUpdateStringTable`, `SvcPacketEntities`, and `SvcServerInfo` internally. Everything else is forwarded to `visitor.on_packet()`.
6. **Entity updates** -- `SvcPacketEntities` triggers `CREATE`, `UPDATE`, or `DELETE` delta events per entity. Each fires `visitor.on_entity()`.
7. **Tick boundary** -- After each command where the tick changes, `visitor.on_tick_end()` fires.

**Tick interval note:** Deadlock runs at 1/60s ticks (Dota 2 runs at 1/30s). `ctx.tick_interval()` returns the correct value after `SvcServerInfo` is processed. Full packet snapshots occur every 900 ticks in Deadlock (vs 1800 in Dota 2).

---

## The Visitor Trait

Implement `Visitor` on your struct to receive parse events. All methods have async signatures and default no-op implementations -- only override what you need.

```rust
pub trait Visitor {
    type Error: core::error::Error + Send + Sync + 'static;

    // Fires on every entity CREATE, UPDATE, or DELETE
    fn on_entity(&mut self, ctx: &Context, delta_header: DeltaHeader, entity: &Entity)
        -> impl Future<Output = Result<(), Self::Error>> + Send + Sync;

    // Fires for every raw demo command (before parsing)
    fn on_cmd(&mut self, ctx: &Context, cmd_header: &CmdHeader, data: &[u8])
        -> impl Future<Output = Result<(), Self::Error>> + Send + Sync;

    // Fires for every sub-message inside a packet (this is where user messages arrive)
    fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8])
        -> impl Future<Output = Result<(), Self::Error>> + Send + Sync;

    // Fires at the end of each tick
    fn on_tick_end(&mut self, ctx: &Context)
        -> impl Future<Output = Result<(), Self::Error>> + Send + Sync;
}
```

---

## How to Subscribe to a Citadel User Message

Match `packet_type` in `on_packet` against the `CitadelUserMessageIds` enum value, then decode with prost.

```rust
use haste::parser::{Context, Parser, Visitor};
use haste::valveprotos::deadlock::{CitadelUserMessageIds, CCitadelUserMsgBossKilled};
use haste::valveprotos::prost::Message;

struct MyVisitor;

impl Visitor for MyVisitor {
    type Error = anyhow::Error;

    async fn on_packet(&mut self, ctx: &Context, packet_type: u32, data: &[u8])
        -> Result<(), Self::Error>
    {
        if packet_type == CitadelUserMessageIds::KEUserMsgBossKilled as u32 {
            let msg = CCitadelUserMsgBossKilled::decode(data)?;
            // msg.gametime, msg.objective_team, msg.bosses_remaining, etc.
        }
        Ok(())
    }
}
```

The `deadlock-api/haste` fork uses **async** Visitor methods (the original `blukai/haste` used sync). Our parser (`parser/Cargo.toml`) depends on `blukai/haste` directly -- check which variant is in use before writing impls.

**Note on enum variant naming:** prost generates Rust enum variants from proto names using PascalCase with a `K` prefix. `k_EUserMsg_BossKilled` becomes `KEUserMsgBossKilled`. Use IDE autocomplete or search the generated types to confirm exact names.

---

## How to Subscribe to Multiple Messages (Handler Pattern)

The `blukai/haste` examples include a `messagehandler-experiment` that shows a composable handler pattern -- register typed handlers by message ID without manually dispatching in `on_packet`:

```rust
// From examples/messagehandler-experiment/
let mut visitor = HandlerVisitor::with_state(state)
    .with(
        CitadelUserMessageIds::KEUserMsgHeroKilled as u32,
        hero_killed,  // fn(&mut State, &Context, &CCitadelUserMsgHeroKilled) -> Result<()>
    )
    .with(
        CitadelUserMessageIds::KEUserMsgBossKilled as u32,
        boss_killed,
    );
```

`HandlerVisitor` is defined in the example, not in the library itself -- copy or adapt it into the dashJump parser if needed.

---

## How to Subscribe to Entity Events

Match on the entity's serializer name hash in `on_entity`:

```rust
use haste::entities::{DeltaHeader, Entity, fkey_from_path};
use haste::fxhash;

const PLAYER_PAWN: u64 = fxhash::hash_bytes(b"CCitadelPlayerPawn");

async fn on_entity(&mut self, ctx: &Context, delta: DeltaHeader, entity: &Entity)
    -> Result<(), Self::Error>
{
    if entity.serializer_name_heq(PLAYER_PAWN) {
        // read a field by path
        const HEALTH_KEY: u64 = fkey_from_path(&["m_iHealth"]);
        let health: i32 = entity.get_value(&HEALTH_KEY).unwrap_or(0);
    }
    Ok(())
}
```

`fkey_from_path` computes a compile-time hash from a field path. The hash is determined by `send_node + var_name` in the proto definition, not by the full inspector-displayed hierarchy.

**CBodyComponent path gotcha:** haste-inspector displays position sub-fields as a 4-level path (`["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]`), but the stored key hash uses only the 2-level path:
```rust
// Correct -- matches the stored key hash
const CELL_X_KEY: u64 = fkey_from_path(&["CBodyComponent", "m_cellX"]);

// Wrong -- returns None every tick despite appearing correct in haste-inspector
const CELL_X_KEY: u64 = fkey_from_path(&["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]);
```
Validated in `parser/src/utils/entity_position.rs`. Rule: when a `get_value` returns `None` on every tick despite the field being visible in haste-inspector, try the 2-level `[send_node, var_name]` path.

---

## Context -- Available State During Callbacks

`ctx` is available in all Visitor callbacks:

| Method | Returns | Notes |
|--------|---------|-------|
| `ctx.tick()` | `i32` | Current tick. `-1` before game start. |
| `ctx.tick_interval()` | `f32` | Seconds per tick. 1/60 for Deadlock. |
| `ctx.entities()` | `Option<&EntityContainer>` | None until first packet entities arrive. |
| `ctx.string_tables()` | `Option<&StringTableContainer>` | Includes `EntityNames` table. |
| `ctx.serializers()` | `Option<&FlattenedSerializerContainer>` | Entity field schema. |
| `ctx.entity_classes()` | `Option<&EntityClasses>` | Class ID to class name mapping. |

Convert tick to game time: `tick as f32 * ctx.tick_interval()`.

---

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `Parser<D, V>` | `parser.rs` | Main parser. Generic over stream and visitor. |
| `Context` | `parser.rs` | Game state snapshot passed to all visitor callbacks. |
| `Visitor` | `parser.rs` | Trait to implement for receiving events. |
| `DemoFile<R>` | `demofile.rs` | File-based `DemoStream`. Wrap with `BufReader` for performance. |
| `Entity` | `entities.rs` | A single networked game entity with field lookup via hashed path keys. |
| `DeltaHeader` | `entities.rs` | `CREATE`, `UPDATE`, or `DELETE` -- which kind of entity event this is. |
| `FieldValue` | `fieldvalue.rs` | Enum over all possible entity field types: `I64`, `U64`, `F32`, `Bool`, `Vector3`, `Vector2`, `Vector4`, `QAngle`, `String`. |
| `EntityContainer` | `entities.rs` | Collection of all live entities, indexed by entity index. |
| `StringTableContainer` | `stringtables.rs` | Named string tables (e.g. `EntityNames`). |

**Entity coordinate decoding:** Deadlock uses a cell-based coordinate system. Use `deadlock_coord_from_cell(cell: u16, vec: f32) -> f32` (from `haste::entities`) to reconstruct world positions from the paired `m_cellX`/`m_vecX` fields.

---

## Feature Flags

| Flag | What it enables |
|------|----------------|
| `deadlock` | Deadlock protobuf types + `deadlock_coord_from_cell` utility |
| `dota2` | Dota 2 protobuf types |
| `broadcast` | HTTP broadcast stream support (`haste_broadcast` crate) |

Our parser uses `features = ["deadlock", "preserve-metadata"]` (see `parser/Cargo.toml`).

---

## Known Limitations / Quirks

- **No examples in the fork.** The `deadlock-api/haste` fork has no `examples/` directory. Examples live in `blukai/haste`. Our `Cargo.toml` points at `blukai/haste` directly.
- **`Symbol` drops string -- no entity name at runtime.** The `blukai/haste` fork stored both `hash: u64` and `str: String` on `Symbol`, allowing `entity.serializer().serializer_name.str` to print the entity class name. The `deadlock-api/haste` fork drops the string and only stores `hash: u64`. To get human-readable names at runtime, maintain a reverse-lookup table from hash to string (see `parser/src/entities/mod.rs::entity_name_for_hash`). Unknown entities without a lookup entry return "UNKNOWN" -- use the hash to identify via haste-inspector.
- **Async Visitor.** The `deadlock-api` fork changed `Visitor` methods to async. The upstream `blukai/haste` uses sync methods. Check which branch your dependency points to -- method signatures differ.
- **Public API is unstable.** The README explicitly warns the API can change dramatically.
- **Many `unsafe` blocks.** The library uses unsafe for performance; safe wrappers are limited.
- **`String` fields may not be valid UTF-8.** Use `String::from_utf8_lossy()` rather than direct conversion. The `m_sHeroBuildSerialized` field on `CCitadelPlayerPawn` is a known offender.
- **No field list on entity update.** `on_entity` fires with the full entity state; there is no list of which fields changed in this tick (noted as a TODO in the source).
- **Seeking resets state.** `run_to_tick()` resets entities, string tables, and instance baseline, then re-processes from the last full packet snapshot.
- **`EntityNames` string table.** Entity class names are looked up via this table using `m_pEntity.m_nameStringableIndex` as the key -- not directly on the entity struct.

---

## Migration Notes: blukai/haste → deadlock-api/haste

**Completed:** March 2026 (commit: "migrate from blukai/haste to deadlock-api/haste fork")

Key changes required when migrating to the deadlock-api fork:

- **`CMsgMatchMetaDataContents` removed.** Use `CMsgMatchMetaDataContentsPatched` directly. The old type name does not compile with the deadlock-api valveprotos.
- **`prost` must be a direct dependency at `0.14.3`.** The fork does not re-export prost. Add `prost = "0.14.3"` to `Cargo.toml`. No feature flags on the haste dep are needed -- the fork handles valveprotos feature selection internally.
- **`DeltaHeader::LEAVE` variant added.** Any exhaustive match on `DeltaHeader` must include a `_ => {}` or explicit `DeltaHeader::LEAVE => {}` arm, or the build fails.
- **PostMatch decode path.** After `CMsgMatchMetaDataContentsPatched::decode`, access `match_paths` as `meta.match_info.map(|i| i.match_paths)`.
- **`src/bin/*.rs` binaries are self-contained.** Binaries in `src/bin/` cannot import from `src/main.rs`'s module tree (no library target). Duplicate haste imports directly in the binary and implement a minimal `Visitor` there.

---

## Integration Notes for dashJump

Our parser (`parser/src/replay_parser.rs`) already uses `blukai/haste` with the `deadlock` feature. The integration pattern is:

1. Implement `Visitor` on a struct that holds accumulator state (e.g. `Vec<CreepSnapshot>`).
2. Subscribe to user messages in `on_packet` by matching `CitadelUserMessageIds` variants.
3. Subscribe to entity state in `on_entity` by matching serializer name hashes (`fxhash::hash_bytes`).
4. Use `ctx.tick()` for timing and `deadlock_coord_from_cell` for positions.
5. After `parser.run_to_end()`, consume state from the visitor struct.

For game phase detection, the recommended approach is to subscribe to both `MidBossSpawned` (ID 349, fires when sinners spawn) and `BossKilled` (ID 347, fires when any boss dies) in `on_packet`, recording `ctx.tick()` at each event to establish phase boundaries.

**Parser coding convention: do not cast proto field types to domain types.** Match the field type in domain structs to what the proto/haste API returns. If a cast appears necessary, that is a signal to change the domain struct's field type, not add a cast. Example: `CCitadelUserMessageDamage.pre_damage` and `damage_absorbed` changed from `int32` (deprecated tags 2, 10) to `float32` (tags 27, 28). Code casting `msg.pre_damage() as i32` silently truncated floats. The fix was changing `DamageRecord.pre_damage` and `DamageRecord.damage_absorbed` to `f32`, not adding a cast. See `parser/src/domain/damage.rs`.

For entity-based features (solo time, creep tracking, player positions), filter in `on_entity` by serializer name and read fields by hashed path key. Use `on_tick_end` to take periodic snapshots rather than processing every update individually.
