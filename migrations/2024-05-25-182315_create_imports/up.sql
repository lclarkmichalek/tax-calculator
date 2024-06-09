CREATE TABLE platforms (
    id TEXT PRIMARY KEY NOT NULL,
    description TEXT NOT NULL,
    url TEXT NOT NULL
);

-- The source of the financial transaction logs imported
CREATE TABLE imports (
    -- Id is the sha256 of the imported file
    id TEXT PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    platform_id TEXT NOT NULL,
    generation_date_unix_timestamp_seconds BIGINT NOT NULL,
    FOREIGN KEY(platform_id) REFERENCES platforms(id) ON DELETE CASCADE
);

CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    platform_id TEXT NOT NULL,
    import_id TEXT NOT NULL,
    FOREIGN KEY(platform_id) REFERENCES platforms(id) ON DELETE CASCADE
    FOREIGN KEY(import_id) REFERENCES imports(id) ON DELETE CASCADE
);

-- The financial transaction recorded
CREATE TABLE transactions (
    id INTEGER PRIMARY KEY,
    execution_time_unix_timestamp_seconds BIGINT NOT NULL,
    ticker_symbol VARCHAR NOT NULL,
    unit_quantity DECIMAL(8, 2) NOT NULL,
    cost_per_unit DECIMAL(8, 2) NOT NULL,
    currency_symbol TEXT NOT NULL,
    account_id TEXT NOT NULL,
    import_id TEXT NOT NULL,
    FOREIGN KEY(account_id) REFERENCES accounts(id),
    FOREIGN KEY(import_id) REFERENCES imports(id) ON DELETE CASCADE
);
