import { MidBossData } from '../../domain/midBoss';
import {
  currentMidBossHealth,
  findActiveCycle,
  rejuvClaimsForCompletedKill,
} from '../../services/midBoss/selectors';

const TEAM_COLORS: Record<number, string> = {
  2: '#FF8C00', // Amber
  3: '#4169E1', // Sapphire
};

const TEAM_LABELS: Record<number, string> = {
  2: 'Amber',
  3: 'Sapphire',
};

interface MidBossHealthBarProps {
  midBoss: MidBossData;
  currentSec: number;
}

const MidBossHealthBar = ({ midBoss, currentSec }: MidBossHealthBarProps) => {
  // A null max_health means the mid-boss never spawned in this match --
  // nothing to display at all, not even a rejuv claim fallback.
  if (midBoss.max_health === null) return null;

  const cycle = findActiveCycle(midBoss, currentSec);
  if (!cycle) return <RecentClaim midBoss={midBoss} currentSec={currentSec} />;

  const health = currentMidBossHealth(midBoss, cycle, currentSec);
  if (health === null) return null;

  const pct = Math.max(0, Math.min(1, health / midBoss.max_health));

  return (
    <div className='mx-auto mt-2 w-full max-w-md md:max-w-lg lg:max-w-xl'>
      <div className='flex items-center justify-between text-xs font-semibold text-gray-700'>
        <span>Mid-Boss</span>
        <span>
          {health.toLocaleString()} / {midBoss.max_health.toLocaleString()}
        </span>
      </div>
      <div
        className='h-3 w-full overflow-hidden rounded bg-gray-300'
        role='progressbar'
        aria-label='Mid-Boss health'
        aria-valuemin={0}
        aria-valuemax={midBoss.max_health}
        aria-valuenow={health}
      >
        <div
          className='h-full rounded bg-red-600 transition-[width] duration-150'
          style={{ width: `${pct * 100}%` }}
        />
      </div>
    </div>
  );
};

// After the most recent kill, briefly show who claimed the rejuv(s). Keeps
// the UI alive for a beat after the bar disappears so the user registers the
// outcome -- per the plan, this is "near the minimap icon location or as an
// overlay after the icon disappears".
const RecentClaim = ({
  midBoss,
  currentSec,
}: MidBossHealthBarProps) => {
  // Find the most recent kill before `currentSec`. We don't care which
  // spawn_cycle it belongs to -- just show the last completed claim.
  const recentKill = [...midBoss.kill_events]
    .filter((k) => currentSec >= k.matchtime_s)
    .sort((a, b) => b.matchtime_s - a.matchtime_s)[0];
  if (!recentKill) return null;

  const claims = rejuvClaimsForCompletedKill(
    midBoss.rejuv_events,
    recentKill,
    currentSec
  );
  if (claims.total === 0) return null;

  const entries = Object.entries(claims.byTeam)
    .map(([teamStr, count]) => ({ team: Number(teamStr), count }))
    .sort((a, b) => b.count - a.count);

  return (
    <div
      className='mx-auto mt-2 flex w-full max-w-md items-center justify-center gap-3 text-xs font-semibold md:max-w-lg lg:max-w-xl'
      aria-label='Mid-Boss rejuv claims'
    >
      <span className='text-gray-600'>Rejuv claimed:</span>
      {entries.map(({ team, count }) => (
        <span
          key={team}
          style={{ color: TEAM_COLORS[team] ?? '#000' }}
        >
          {TEAM_LABELS[team] ?? `Team ${team}`}: {count}
        </span>
      ))}
    </div>
  );
};

export default MidBossHealthBar;
