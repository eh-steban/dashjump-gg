import React from 'react';
import { ParsedMatchData } from '../../domain/matchAnalysis';
import { Region } from '../../domain/region';
import { regions } from '../../data/regions';
import {
  PlayerData,
  PlayerMatchData,
  DRTypeAggregateBySec,
} from '../../domain/player';
import pointInPolygon from 'point-in-polygon';

interface PlayerCardsProps {
  players: PlayerData[];
  perPlayerData: Record<string, PlayerMatchData>;
  currentTick: number;
  matchData: ParsedMatchData;
  normalizePosition: (x: number, y: number) => { normX: number; normY: number };
}

function getPlayerRegionLabels(x: number, y: number): string[] {
  const foundRegions: string[] = regions
    .filter((region: Region) => pointInPolygon([x, y], region.polygon))
    .map<string>((region): string => {
      return region.label ? region.label : 'None';
    });
  return foundRegions;
}

// TODO: Add typechecking?
const PlayerCards: React.FC<PlayerCardsProps> = ({
  players,
  perPlayerData,
  currentTick,
  normalizePosition,
}) => {
  return (
    <>
      <h3 className='mt-0 mb-2'>Player Info</h3>
      <div
        title='playerCards'
        className='grid w-full grid-cols-2 gap-1 border-0 bg-transparent p-0'
      >
        {players.map((player) => {
          const customId = Number(player.custom_id);
          const posDmgdata = perPlayerData[customId];
          const playerPosition = posDmgdata.positions[currentTick];
          const victimDamageMap = posDmgdata.damage[currentTick];
          const team = player.team;
          const heroName = player.hero.name || `Hero ${player.hero_id}`;
          const heroImg = player.hero.images?.icon_hero_card_webp;
          const health = 0;
          // const health = playerPathState.health[currentTick];
          const { normX, normY } = normalizePosition(
            playerPosition.x,
            playerPosition.y
          );
          const regionLabels: string[] = getPlayerRegionLabels(normX, normY);

          // Summarize: victimId -> (type -> totalDamage)
          // FIXME: Just for display purposes, but this should be done
          // in our backend or data processing steps ideally
          const totalsByVictim: Record<
            string,
            Record<number, DRTypeAggregateBySec>
          > = {};
          for (const [victimId, records] of Object.entries(victimDamageMap)) {
            const aggByType: Record<number, DRTypeAggregateBySec> = {};
            for (const rec of records) {
              const t = rec.type ?? -1;
              // FIXME: Not all the fields below are being displayed/used.
              // Need to update this to show this data somewhere.
              const entry = (aggByType[t] ??= {
                type: t,
                agg_damage: 0,
                agg_pre_damage: 0,
                citadel_type: rec.citadel_type ?? 0,
                entindex_inflictor: rec.entindex_inflictor ?? 0,
                entindex_ability: rec.entindex_ability ?? 0,
                agg_damage_absorbed: 0,
                victim_health_max: 0,
                victim_health_new: 0,
                flags: rec.flags ?? 0,
                ability_id: rec.ability_id ?? 0,
                attacker_class: rec.attacker_class ?? 0,
                victim_class: rec.victim_class ?? 0,
                victim_shield_max: 0,
                agg_victim_shield_new: 0,
                agg_hits: 0,
                agg_health_lost: 0,
              });
              entry.agg_damage += rec.damage ?? 0;
              entry.agg_pre_damage += rec.pre_damage ?? 0;
              entry.agg_damage_absorbed += rec.damage_absorbed ?? 0;
              entry.agg_victim_shield_new += rec.victim_shield_new ?? 0;
              entry.agg_hits += rec.hits ?? 0;
              entry.agg_health_lost += rec.health_lost ?? 0;
            }
            totalsByVictim[victimId] = aggByType;
          }

          return (
            <div
              key={`player-card-${player.lobby_player_slot}`}
              className='flex h-70 flex-row items-stretch gap-[0.025rem] rounded-lg border border-gray-300 bg-white px-3 py-2 shadow-sm'
            >
              {heroImg && (
                <img
                  src={heroImg}
                  alt={heroName}
                  className='h-16 w-16 rounded-2xl border border-gray-300 bg-gray-200 object-cover'
                />
              )}
              <div className='flex flex-1 flex-col gap-0.5'>
                <div className='mb-0.5 text-[1.05em] font-semibold'>
                  {heroName}{' '}
                  <span className='text-[0.95em] font-normal text-gray-500'>
                    (Name: {player.name}, Slot: {player.lobby_player_slot},
                    Team: {team})
                  </span>
                </div>
                <div className='flex flex-col gap-2.5 text-[0.97em]'>
                  <div>
                    <strong>Health:</strong>{' '}
                    {health !== undefined ? health : '-'}
                  </div>
                  <div>
                    <strong>Current Region:</strong> {regionLabels.join(', ')}
                  </div>
                  <div>
                    {Object.keys(victimDamageMap).length === 0 ?
                      <strong>Out of combat</strong>
                    : <>
                        <strong>Victims:</strong>
                        <ul className='m-0 pl-[18px]'>
                          {Object.entries(totalsByVictim).map(
                            ([victimId, typeTotals]) => {
                              const victimPlayer = players.find(
                                (p) => String(p.custom_id) === victimId
                              );
                              const victimName =
                                victimPlayer ?
                                  victimPlayer.name
                                : `Victim ${victimId}`;
                              return (
                                <li key={victimId}>
                                  {victimName}
                                  <ul className='m-0 pl-[18px]'>
                                    {Object.entries(typeTotals).map(
                                      ([type, DRAgg]) => (
                                        <li key={type}>
                                          Type {type}: {DRAgg.agg_damage}
                                        </li>
                                      )
                                    )}
                                  </ul>
                                </li>
                              );
                            }
                          )}
                        </ul>
                        {/*
                          FIXME: The below block is being kept for debugging in dev.
                          This should be removed later.
                        */}
                        {/* <ul className='m-0 pl-[18px]'>
                          {Object.entries(victimDamageMap).map(
                            ([victimIdx, damageRecords]) => {
                              const victimPlayer =
                                players[Number(victimIdx)];
                              let victimName =
                                victimPlayer ?
                                  victimPlayer.name
                                : `Victim ${victimIdx}`;
                              return (
                                <li key={victimIdx}>
                                  {victimName}
                                  <ul className='m-0 pl-[18px]'>
                                    {damageRecords.map((rec, idx) => (
                                      <li key={idx}>
                                        Damage: {rec.damage} | Type: {rec.type}{" "}
                                        | ability_id: {rec.ability_id} |
                                        attacker_class: {rec.attacker_class} |
                                        citadel_type: {rec.citadel_type} |
                                        victim_class: {rec.victim_class}
                                      </li>
                                    ))}
                                  </ul>
                                </li>
                              );
                            }
                          )}
                        </ul> */}
                      </>
                    }
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
};

export default PlayerCards;
