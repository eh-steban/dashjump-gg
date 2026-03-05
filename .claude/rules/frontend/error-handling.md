---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# Frontend Error Handling

React/TypeScript error handling patterns and standards.

## Error Types

Define typed errors for consistent handling:

```typescript
// domain/errors.ts
interface AppError {
  type: 'network' | 'api' | 'validation' | 'unknown';
  message: string;
  statusCode?: number;
  retryable: boolean;
}

function toAppError(err: unknown): AppError {
  if (err instanceof Response || (err && typeof err === 'object' && 'status' in err)) {
    const status = (err as Response).status;
    return {
      type: 'api',
      message: `Request failed (${status})`,
      statusCode: status,
      retryable: status >= 500,
    };
  }
  if (err instanceof TypeError && err.message.includes('fetch')) {
    return { type: 'network', message: 'Network error', retryable: true };
  }
  return { type: 'unknown', message: String(err), retryable: false };
}
```

## Hook Error State

Use typed error state instead of `unknown`:

```typescript
// ✅ Good - typed error state
const [error, setError] = useState<AppError | null>(null);

// ❌ Bad - untyped error
const [error, setError] = useState<unknown>(null);
```

## Error Boundaries

Implement React error boundaries for critical components:

```tsx
// components/common/ErrorBoundary.tsx
interface Props {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends React.Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('Error boundary caught:', error, info.componentStack);
    // Future: Send to error tracking service
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback ?? <ErrorFallback error={this.state.error} />;
    }
    return this.props.children;
  }
}
```

Use error boundaries around:
- Page-level components
- Visualization components (charts, canvas)
- Any component that processes external data

## Graceful Degradation

### Stale-While-Error (Already Implemented)

Continue using the `allowStaleOnError` pattern:

```typescript
const data = await fetchMatchAnalysis(matchId, { allowStaleOnError: true });
```

### Partial Data Display

Show available data even when some parts fail:

```tsx
{matchData && <MatchOverview data={matchData} />}
{timelineError ? (
  <TimelineError onRetry={refetchTimeline} />
) : (
  <Timeline data={timelineData} />
)}
```

### Silent Failures for Non-Critical Operations

Continue the existing pattern for localStorage:

```typescript
function saveToStorage(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Silently ignore - localStorage may be full or disabled
  }
}
```

## User-Facing Error Messages

| Error Type | User Message | Show Retry? |
|------------|--------------|-------------|
| Network | "Unable to connect. Check your connection." | Yes |
| 404 | "Match not found. Check the match ID." | No |
| 500 | "Something went wrong. Please try again." | Yes |
| Timeout | "Request timed out. The server may be busy." | Yes |

## Error UI Components

```tsx
// components/common/ErrorMessage.tsx
interface ErrorMessageProps {
  error: AppError;
  onRetry?: () => void;
}

const ErrorMessage: React.FC<ErrorMessageProps> = ({ error, onRetry }) => (
  <div className="p-4 bg-red-50 border border-red-200 rounded-lg">
    <p className="text-red-800">{getUserMessage(error)}</p>
    {error.retryable && onRetry && (
      <button
        onClick={onRetry}
        className="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
      >
        Try Again
      </button>
    )}
  </div>
);
```

## API Error Handling

```typescript
async function fetchMatchAnalysis(matchId: number): Promise<MatchAnalysis> {
  const response = await fetch(`/api/match/analysis/${matchId}`);

  if (!response.ok) {
    const text = await response.text().catch(() => 'Unknown error');
    throw new Error(`(${response.status}): ${text}`);
  }

  return response.json();
}
```

## Best Practices

1. **Always handle errors** — Every async operation needs error handling
2. **Type your errors** — Use `AppError` instead of `unknown`
3. **Show user-friendly messages** — Never display raw error text
4. **Enable retry for transient failures** — Network and 5xx errors are retryable
5. **Log errors with context** — Include relevant IDs for debugging
6. **Use error boundaries** — Prevent entire app crashes from component errors
