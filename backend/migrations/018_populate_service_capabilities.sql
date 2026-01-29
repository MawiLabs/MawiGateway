-- Migration: Populate service input_modalities and output_modalities from assigned models
-- This computes capabilities for all existing services based on their assigned models
-- Note: models table has a single 'modality' column, we use it for both input and output

-- For most current services (text models), both input and output will be 'text'
-- In the future, models with different input/output (like veo3: text→video) will need 
-- separate input_modality and output_modality columns in the models table

UPDATE services
SET 
    input_modalities = (
        SELECT COALESCE(json_agg(DISTINCT m.modality)::text, '[]')
        FROM service_models sm
        JOIN models m ON sm.model_id = m.id
        WHERE sm.service_name = services.name
        AND m.modality IS NOT NULL
    ),
    output_modalities = (
        SELECT COALESCE(json_agg(DISTINCT m.modality)::text, '[]')
        FROM service_models sm
        JOIN models m ON sm.model_id = m.id
        WHERE sm.service_name = services.name
        AND m.modality IS NOT NULL
    )
WHERE EXISTS (
    SELECT 1 FROM service_models WHERE service_name = services.name
);
