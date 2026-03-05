---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# Frontend Observability

React/TypeScript logging and monitoring guidelines.

## Current State

Minimal logging — keep console output lean and intentional.

## Console Logging Guidelines

### When to Log

```typescript
// ✅ Log API errors with sanitized context
console.error('Failed to load match', { matchId, status: err.status });

// ✅ Log unexpected states that help debugging
console.warn('Received empty damage array for match', matchId);

// ❌ Don't log routine operations
console.log('Fetching match data...');

// ❌ Don't log sensitive or verbose data
console.log('API response:', fullResponse);
console.log('User session:', sessionData);
```

### Log Levels

| Method | Use For |
|--------|---------|
| `console.error()` | Errors requiring attention, caught exceptions |
| `console.warn()` | Recoverable issues, deprecated patterns |
| `console.log()` | Development debugging (remove before commit) |

### Development vs Production

```typescript
if (process.env.NODE_ENV === 'development') {
  console.log('Debug info:', data);
}
```

## Performance Monitoring

### Web Vitals

The `web-vitals` package is installed. Enable it for performance insights:

```typescript
// index.tsx
import { reportWebVitals } from './reportWebVitals';

reportWebVitals((metric) => {
  // Development: log to console
  if (process.env.NODE_ENV === 'development') {
    console.log(metric.name, metric.value);
  }

  // Future: Send to analytics service
  // sendToAnalytics(metric);
});
```

### Slow Render Detection

In development, log slow renders:

```typescript
function useRenderTiming(componentName: string) {
  useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return;

    const start = performance.now();
    return () => {
      const duration = performance.now() - start;
      if (duration > 100) {
        console.warn(`Slow render: ${componentName} took ${duration.toFixed(0)}ms`);
      }
    };
  });
}
```

## Error Logging

### Centralized Error Logging (Future)

```typescript
// utils/errorTracking.ts
export function captureError(
  error: Error,
  context?: Record<string, unknown>
): void {
  // Phase 1: Console logging
  console.error('Error captured:', error.message, context);

  // Phase 2: Sentry integration (future)
  // Sentry.captureException(error, { extra: context });
}

// Usage
try {
  await fetchMatchAnalysis(matchId);
} catch (error) {
  captureError(error as Error, { matchId, component: 'MatchAnalysis' });
}
```

### Error Boundary Logging

```typescript
componentDidCatch(error: Error, info: React.ErrorInfo) {
  console.error('Error boundary caught:', {
    error: error.message,
    componentStack: info.componentStack,
  });

  // Future: captureError(error, { componentStack: info.componentStack });
}
```

## What NOT to Log

- User authentication tokens or session data
- Personal information
- Full API responses (use DEBUG builds if needed)
- High-frequency events (mousemove, scroll in loops)

## Network Request Logging

For API debugging, use browser DevTools Network tab instead of console logging.

Only log errors:

```typescript
async function fetchWithLogging<T>(url: string): Promise<T> {
  try {
    const response = await fetch(url);
    if (!response.ok) {
      console.error('API error:', { url, status: response.status });
      throw new Error(`HTTP ${response.status}`);
    }
    return response.json();
  } catch (error) {
    console.error('Network error:', { url, error });
    throw error;
  }
}
```

## Future: Analytics Integration

```typescript
// utils/analytics.ts (future)
interface AnalyticsEvent {
  name: string;
  properties?: Record<string, unknown>;
}

export function trackEvent(event: AnalyticsEvent): void {
  // Phase 1: Console in development
  if (process.env.NODE_ENV === 'development') {
    console.log('Analytics:', event);
  }

  // Phase 2: Send to analytics service
  // analytics.track(event.name, event.properties);
}

// Usage
trackEvent({
  name: 'match_analyzed',
  properties: { matchId, duration: matchDuration },
});
```
