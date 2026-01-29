-- Migration 026: Fix model cost column types to DOUBLE PRECISION
-- SQLx decoding fails when trying to map REAL (f32) to Rust f64.

ALTER TABLE models 
  ALTER COLUMN cost_per_1k_tokens TYPE DOUBLE PRECISION,
  ALTER COLUMN cost_per_1k_input_tokens TYPE DOUBLE PRECISION,
  ALTER COLUMN cost_per_1k_output_tokens TYPE DOUBLE PRECISION;
