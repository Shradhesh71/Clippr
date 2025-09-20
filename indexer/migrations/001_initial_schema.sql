-- Create custom enums
CREATE TYPE indexer_status AS ENUM ('starting', 'running', 'stopped', 'error');
CREATE TYPE subscription_type AS ENUM ('account', 'transaction', 'both');
CREATE TYPE balance_change_type AS ENUM ('increase', 'decrease', 'swapIn', 'swapOut', 'transfer', 'unknown');
CREATE TYPE transaction_type AS ENUM ('transfer', 'swap', 'stake', 'vote', 'createAccount', 'closeAccount', 'other');

-- Indexer state table
CREATE TABLE indexer_states (
    id VARCHAR PRIMARY KEY,
    subscribed_keys JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_processed_slot BIGINT NOT NULL DEFAULT 0,
    status indexer_status NOT NULL DEFAULT 'starting',
    total_subscriptions INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Subscribed keys table
CREATE TABLE subscribed_keys (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    public_key VARCHAR(44) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    subscription_type subscription_type NOT NULL DEFAULT 'both',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    UNIQUE(user_id, public_key)
);

-- Balance updates table
CREATE TABLE balance_updates (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    public_key VARCHAR(44) NOT NULL,
    mint_address VARCHAR(44) NOT NULL,
    old_balance DECIMAL(20,9) NOT NULL,
    new_balance DECIMAL(20,9) NOT NULL,
    change_amount DECIMAL(20,9) NOT NULL,
    change_type balance_change_type NOT NULL,
    transaction_signature VARCHAR(88),
    slot BIGINT NOT NULL,
    block_time TIMESTAMP WITH TIME ZONE,
    processed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Transaction events table
CREATE TABLE transaction_events (
    id VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    public_key VARCHAR(44) NOT NULL,
    transaction_signature VARCHAR(88) NOT NULL,
    transaction_type transaction_type NOT NULL,
    slot BIGINT NOT NULL,
    block_time TIMESTAMP WITH TIME ZONE,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    program_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
    processed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    UNIQUE(transaction_signature, public_key)
);

-- Indexer statistics table
CREATE TABLE indexer_stats (
    id VARCHAR PRIMARY KEY,
    total_keys_monitored INTEGER NOT NULL DEFAULT 0,
    total_balance_updates BIGINT NOT NULL DEFAULT 0,
    total_transactions BIGINT NOT NULL DEFAULT 0,
    last_processed_slot BIGINT NOT NULL DEFAULT 0,
    avg_processing_time_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    errors_last_hour INTEGER NOT NULL DEFAULT 0,
    uptime_seconds BIGINT NOT NULL DEFAULT 0,
    recorded_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_indexer_states_updated_at BEFORE UPDATE ON indexer_states FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_subscribed_keys_updated_at BEFORE UPDATE ON subscribed_keys FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create indexes for better performance
CREATE INDEX idx_subscribed_keys_public_key ON subscribed_keys (public_key);
CREATE INDEX idx_subscribed_keys_user_id ON subscribed_keys (user_id);
CREATE INDEX idx_subscribed_keys_active ON subscribed_keys (is_active);

CREATE INDEX idx_balance_updates_user_id ON balance_updates (user_id);
CREATE INDEX idx_balance_updates_public_key ON balance_updates (public_key);
CREATE INDEX idx_balance_updates_mint ON balance_updates (mint_address);
CREATE INDEX idx_balance_updates_slot ON balance_updates (slot);
CREATE INDEX idx_balance_updates_processed_at ON balance_updates (processed_at);
CREATE INDEX idx_balance_updates_signature ON balance_updates (transaction_signature);

CREATE INDEX idx_transaction_events_user_id ON transaction_events (user_id);
CREATE INDEX idx_transaction_events_public_key ON transaction_events (public_key);
CREATE INDEX idx_transaction_events_signature ON transaction_events (transaction_signature);
CREATE INDEX idx_transaction_events_slot ON transaction_events (slot);
CREATE INDEX idx_transaction_events_type ON transaction_events (transaction_type);
CREATE INDEX idx_transaction_events_processed_at ON transaction_events (processed_at);

CREATE INDEX idx_indexer_stats_recorded_at ON indexer_stats (recorded_at);

-- Insert initial indexer state
INSERT INTO indexer_states (id, status) VALUES ('primary', 'starting');