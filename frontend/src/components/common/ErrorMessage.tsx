import React from 'react';

interface ErrorMessageProps {
  error: Error | string | null;
  title?: string;
  onRetry?: () => void;
  className?: string;
}

/**
 * Reusable error message component with optional retry button.
 *
 * Usage:
 * - Simple error: <ErrorMessage error="Something went wrong" />
 * - With retry: <ErrorMessage error={error} onRetry={refetch} />
 * - Custom title: <ErrorMessage error={error} title="Failed to load match" />
 */
export const ErrorMessage: React.FC<ErrorMessageProps> = ({
  error,
  title = 'Error',
  onRetry,
  className = '',
}) => {
  if (!error) return null;

  const errorMessage = error instanceof Error ? error.message : error;

  // Determine error type and styling based on message content
  const isNotFound = errorMessage.toLowerCase().includes('not found') ||
                     errorMessage.toLowerCase().includes('404');
  const isServerError = errorMessage.toLowerCase().includes('server') ||
                       errorMessage.toLowerCase().includes('502') ||
                       errorMessage.toLowerCase().includes('503');
  const isTimeout = errorMessage.toLowerCase().includes('timeout');

  // Choose appropriate color scheme
  const colorClasses = isNotFound
    ? 'bg-yellow-50 border-yellow-300 text-yellow-800'
    : isTimeout
    ? 'bg-orange-50 border-orange-300 text-orange-800'
    : 'bg-red-50 border-red-300 text-red-800';

  const iconClasses = isNotFound
    ? 'text-yellow-600'
    : isTimeout
    ? 'text-orange-600'
    : 'text-red-600';

  return (
    <div className={`rounded-lg border p-4 ${colorClasses} ${className}`}>
      <div className="flex items-start">
        <div className="flex-shrink-0">
          <svg
            className={`h-5 w-5 ${iconClasses}`}
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            {isNotFound ? (
              // Search icon for not found
              <path
                fillRule="evenodd"
                d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z"
                clipRule="evenodd"
              />
            ) : (
              // Exclamation icon for errors
              <path
                fillRule="evenodd"
                d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                clipRule="evenodd"
              />
            )}
          </svg>
        </div>
        <div className="ml-3 flex-1">
          <h3 className="text-sm font-medium">{title}</h3>
          <div className="mt-2 text-sm">
            <p>{getUserFriendlyMessage(errorMessage)}</p>
          </div>
          {onRetry && (
            <div className="mt-4">
              <button
                onClick={onRetry}
                className={`inline-flex items-center px-3 py-2 text-sm font-medium rounded-md
                  ${isNotFound
                    ? 'bg-yellow-100 hover:bg-yellow-200 text-yellow-800'
                    : isTimeout
                    ? 'bg-orange-100 hover:bg-orange-200 text-orange-800'
                    : 'bg-red-100 hover:bg-red-200 text-red-800'
                  }
                  focus:outline-none focus:ring-2 focus:ring-offset-2
                  ${isNotFound ? 'focus:ring-yellow-500' : isTimeout ? 'focus:ring-orange-500' : 'focus:ring-red-500'}
                `}
              >
                <svg
                  className="-ml-0.5 mr-2 h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  />
                </svg>
                Try Again
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

/**
 * Convert technical error messages to user-friendly text
 */
function getUserFriendlyMessage(message: string): string {
  // Handle specific error patterns
  if (message.includes('Replay URL not found')) {
    return 'This match replay is not available. It may not have been uploaded yet, or the match ID may be incorrect.';
  }

  if (message.includes('502') || message.includes('Bad Gateway')) {
    return 'The parser service is temporarily unavailable. Please try again in a moment.';
  }

  if (message.includes('503') || message.includes('Service Unavailable')) {
    return 'The service is temporarily unavailable. Please try again shortly.';
  }

  if (message.includes('timeout')) {
    return 'The request took too long to complete. The server may be busy, please try again.';
  }

  if (message.includes('404') || message.includes('not found')) {
    return message; // Not found messages are usually already clear
  }

  if (message.includes('Network') || message === 'Failed to fetch') {
    return 'Unable to connect to the server. Please check your internet connection and try again.';
  }

  if (message.includes('(500)')) {
    return 'Something went wrong on the server. Please try again later.';
  }

  // Default: return original message for unrecognized errors
  return message;
}
