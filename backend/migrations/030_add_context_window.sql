-- Add context_window to models for smart pruning
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='models' AND column_name='context_window') THEN
        ALTER TABLE models ADD COLUMN context_window INTEGER DEFAULT 8192;
    END IF;
END $$;
