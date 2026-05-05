CREATE TEMP VIEW changes AS
  SELECT z, x, y,
         p.size AS prev_size,
         c.size AS curr_size,
         c.size - p.size AS delta
  FROM main.sizes p
  JOIN current_head.sizes c USING (z, x, y);

CREATE TEMP VIEW agg AS
  SELECT z,
         COUNT(*) AS tiles,
         SUM(prev_size) AS prev_total,
         SUM(curr_size) AS curr_total,
         SUM(delta) AS delta_total,
         AVG(delta) AS avg_delta,
         MIN(delta) AS best_delta,
         MAX(delta) AS worst_delta
  FROM changes
  GROUP BY z

  UNION ALL

  SELECT -1,
         COUNT(*),
         SUM(prev_size),
         SUM(curr_size),
         SUM(delta),
         AVG(delta),
         MIN(delta),
         MAX(delta)
  FROM changes;

-- Final output
.mode markdown
.headers on

SELECT
  CASE WHEN z = -1 THEN 'ALL' ELSE z END AS zoom,
  tiles,
  prev_total,
  curr_total,
  delta_total,
  ROUND(avg_delta, 0) AS avg_delta,
  ROUND(
    CASE WHEN prev_total=0 THEN 0 ELSE 100.0*delta_total/prev_total END,
    2
  ) AS pct,

  CASE WHEN best_delta < 0 THEN (
    SELECT printf('%d/%d/%d (%+d)', z, x, y, delta)
    FROM (
      SELECT *, ROW_NUMBER() OVER (ORDER BY x, y) rn
      FROM changes c2
      WHERE (agg.z = -1 OR c2.z = agg.z)
        AND delta = best_delta
    ) WHERE rn = 1
  ) END AS best,

  CASE WHEN worst_delta > 0 THEN (
    SELECT printf('%d/%d/%d (%+d)', z, x, y, delta)
    FROM (
      SELECT *, ROW_NUMBER() OVER (ORDER BY x, y) rn
      FROM changes c2
      WHERE (agg.z = -1 OR c2.z = agg.z)
        AND delta = worst_delta
    ) WHERE rn = 1
  ) END AS worst

FROM agg
ORDER BY (z = -1), z;

.print
.print '### Top 3 improvements'
.print

SELECT
  printf('%d/%d/%d', z, x, y),
  prev_size,
  curr_size,
  delta,
  ROUND(CASE WHEN prev_size=0 THEN 0 ELSE 100.0*delta/prev_size END, 2) as pct
FROM changes
WHERE delta < 0
ORDER BY delta ASC, z, x, y
LIMIT 3;

.print
.print '### Top 3 degradations'
.print

SELECT
  printf('%d/%d/%d', z, x, y),
  prev_size,
  curr_size,
  delta,
  ROUND(CASE WHEN prev_size=0 THEN 0 ELSE 100.0*delta/prev_size END, 2) as pct
FROM changes
WHERE delta > 0
ORDER BY delta DESC, z, x, y
LIMIT 3;
