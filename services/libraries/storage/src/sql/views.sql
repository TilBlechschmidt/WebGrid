CREATE TEMP VIEW IF NOT EXISTS Scoring 
AS 
    SELECT
        Path,
        Size,
        (strftime('%s', 'now') - strftime('%s', ModificationTime)) AS SecondsSinceModification,
        (strftime('%s', 'now') - strftime('%s', LastAccessTime)) AS SecondsSinceLastAccess,
        AccessCount,
        Priority,
        {score_formula} AS Score
    FROM Files
    WHERE SecondsSinceModification >= {seconds_since_modification_threshold}
    ORDER BY Score DESC;

CREATE TEMP VIEW IF NOT EXISTS Eviction
AS
    SELECT
        *,
        SUM(Size) OVER (
            ORDER BY Score DESC
            ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
        ) AS CumulativeSize
    FROM Scoring
    ORDER BY Score DESC;