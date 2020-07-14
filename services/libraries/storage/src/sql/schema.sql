CREATE TABLE IF NOT EXISTS Files (
    Path                TEXT PRIMARY KEY NOT NULL,
    
    -- File metadata
    Size                DOUBLE PRECISION NOT NULL DEFAULT 0,
    ModificationTime    TEXT NOT NULL,
    LastAccessTime      TEXT NOT NULL,
    AccessCount         UNSIGNED INTEGER NOT NULL DEFAULT 0,

    -- Other properties
    Priority            INTEGER NOT NULL DEFAULT 0
);