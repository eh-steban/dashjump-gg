import { useEffect, useState, useCallback } from 'react';
import { fetchMatchAnalysis } from '../api/MatchAnalysis';
import { MatchAnalysisResponse } from '../domain/matchAnalysis';

export function useMatchAnalysis(matchId: number) {
  const [data, setData] = useState<MatchAnalysisResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await fetchMatchAnalysis(matchId);
      console.log('[MatchAnalysis] API response:', result);
      setData(result);
    } catch (err) {
      setError(err);
    } finally {
      setLoading(false);
    }
  }, [matchId]);

  useEffect(() => {
    if (matchId == null) return;
    load();
  }, [matchId, load]);

  return { data, loading, error, refetch: load } as const;
}
