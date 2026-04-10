"""Population bootstrap module.

Pulls ACS 5-year county data from the Census Bureau API, with disk
caching and synthetic fallback when no API key is configured.
"""

from sim.bootstrap.population import bootstrap_population

__all__ = ["bootstrap_population"]
