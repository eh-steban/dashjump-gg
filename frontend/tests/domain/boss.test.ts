import { describe, expect, it } from 'vitest';
import { getBossDisplayName, BossSnapshot } from '../../src/domain/boss';

const HASH_GUARDIAN      = '12946736302082733589';
const HASH_WALKER        = '1942975293714691302';
const HASH_BASE_GUARDIAN = '793562361056549792';
const HASH_SHRINE        = '8292725763874089450';
const HASH_PATRON        = '7814756300278693755';

function makeSnapshot(overrides: Partial<BossSnapshot>): BossSnapshot {
  return {
    entity_index: 100,
    custom_id: 21,
    boss_name_hash: HASH_GUARDIAN,
    team: 2,
    lane: 1,
    x: 0,
    y: 0,
    z: 0,
    spawn_time_s: 0,
    max_health: 5500,
    life_state_on_create: 0,
    death_time_s: null,
    life_state_on_delete: null,
    ...overrides,
  };
}

describe('getBossDisplayName', () => {
  it('labels a Guardian with its lane', () => {
    const boss = makeSnapshot({ boss_name_hash: HASH_GUARDIAN, lane: 2 });
    expect(getBossDisplayName(boss)).toBe('Guardian - Lane 2');
  });

  it('labels a Walker with its lane', () => {
    const boss = makeSnapshot({ boss_name_hash: HASH_WALKER, lane: 3, custom_id: 28 });
    expect(getBossDisplayName(boss)).toBe('Walker - Lane 3');
  });

  it('labels a Base Guardian with lane and entity_index', () => {
    const boss = makeSnapshot({
      boss_name_hash: HASH_BASE_GUARDIAN,
      lane: 1,
      entity_index: 124,
      custom_id: 26,
    });
    expect(getBossDisplayName(boss)).toBe('Base Guardian - Lane 1 (124)');
  });

  it('labels a Shrine with lane and entity_index', () => {
    const boss = makeSnapshot({
      boss_name_hash: HASH_SHRINE,
      lane: 2,
      entity_index: 88,
      custom_id: 27,
    });
    expect(getBossDisplayName(boss)).toBe('Shrine - Lane 2 (88)');
  });

  it('labels a Patron without a lane suffix when lane is 0', () => {
    const boss = makeSnapshot({ boss_name_hash: HASH_PATRON, lane: 0, custom_id: 29 });
    expect(getBossDisplayName(boss)).toBe('Patron');
  });

  it('falls back to Boss #<hash> for unknown hashes', () => {
    const boss = makeSnapshot({ boss_name_hash: '9999999999999999999' });
    expect(getBossDisplayName(boss)).toBe('Boss #9999999999999999999 - Lane 1');
  });
});
