const iso8601DurationRegex =
  /^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)(?:\.(\d+))?S)?)?$/;

/**
 * Parse an ISO 8601 duration string into seconds.
 * @param duration The ISO 8601 duration string.
 * @returns The duration in seconds.
 * @example
 * parseISODuration('P1Y2M3DT4H5M6.789S') // 37293756.789
 * parseISODuration('PT1H') // 3600
 * parseISODuration('P1D') // 86400
 * parseISODuration('P1M') // 2629746
 * parseISODuration('P1Y') // 31556952
 * parseISODuration('P1Y2M3DT4H5M6S') // 37293756
 * parseISODuration('PT488.799999S') // 488.799999
 */
export function parseISODuration(duration: string): number {
  const matches = iso8601DurationRegex.exec(duration);

  if (!matches) {
    return 0;
  }

  const [_all, years, months, days, hours, minutes, seconds, milliseconds] =
    matches;

  const totalSeconds =
    (parseInt(years, 10) || 0) * 12 * 30 * 24 * 60 * 60 +
    (parseInt(months, 10) || 0) * 30 * 24 * 60 * 60 +
    (parseInt(days, 10) || 0) * 24 * 60 * 60 +
    (parseInt(hours, 10) || 0) * 60 * 60 +
    (parseInt(minutes, 10) || 0) * 60 +
    (parseInt(seconds, 10) || 0);

  return (
    totalSeconds +
    (parseInt(milliseconds, 10) || 0) / 10 ** (milliseconds?.length || 0)
  );
}

export function formatDuration(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  seconds -= days * 86400;

  const hours = Math.floor(seconds / 3600);
  seconds -= hours * 3600;

  const minutes = Math.floor(seconds / 60);
  seconds -= minutes * 60;

  const parts = [];

  if (days) {
    parts.push(`${days}d`);
  }

  if (hours) {
    parts.push(`${hours}h`);
  }

  if (minutes) {
    parts.push(`${minutes}m`);
  }

  if (seconds) {
    // Round to 2 decimal places
    parts.push(`${Math.round(seconds * 100) / 100}s`);
  }

  return parts.join(" ");
}
