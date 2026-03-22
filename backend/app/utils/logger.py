"""
Centralized logger utility providing consistent logging configuration across the application.
"""
import logging
import sys
from typing import Optional


class LoggerManager:
    _instance: Optional['LoggerManager'] = None
    _initialized: bool = False

    def __new__(cls) -> 'LoggerManager':
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self):
        """Initialize the logger manager (only once)."""
        if not LoggerManager._initialized:
            self._setup_logging()
            LoggerManager._initialized = True

    def _setup_logging(self):
        """Configure the root logger with desired format and handlers."""
        log_format = '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        logging.basicConfig(
            level=logging.INFO,
            format=log_format,
            handlers=[
                logging.StreamHandler(sys.stdout)
            ]
        )

    def get_logger(self, name: str) -> logging.Logger:
        """
        Get a logger instance for the given module name.

        Args:
            name: The name of the module (typically __name__)

        Returns:
            A configured logger instance
        """
        return logging.getLogger(name)


def fmt_duration(ms: float) -> str:
    """Format a duration in milliseconds for log output.

    Examples:
        1846.3  -> '1846 ms (1.8s)'
        90123.0 -> '90123 ms (1m 30s)'
    """
    if ms < 60_000:
        return f"{ms:.0f} ms ({ms / 1000:.1f}s)"
    minutes = int(ms // 60_000)
    seconds = (ms % 60_000) / 1000
    return f"{ms:.0f} ms ({minutes}m {seconds:.0f}s)"


def get_logger(name: str) -> logging.Logger:
    """
    Convenience function to get a logger instance.

    Args:
        name: The name of the module (typically __name__)

    Returns:
        A configured logger instance
    """
    manager = LoggerManager()
    return manager.get_logger(name)
