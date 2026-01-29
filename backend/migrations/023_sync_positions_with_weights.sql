-- One-time migration to sync positions with weights for all services
-- This fixes any existing misalignments

-- For each service, reorder models by weight DESC
WITH ranked_models AS (
    SELECT 
        service_name,
        model_id,
        weight,
        position,
        ROW_NUMBER() OVER (PARTITION BY service_name ORDER BY weight DESC, position ASC) as new_position
    FROM service_models
)
UPDATE service_models 
SET position = (
    SELECT new_position 
    FROM ranked_models 
    WHERE ranked_models.service_name = service_models.service_name 
    AND ranked_models.model_id = service_models.model_id
)
WHERE EXISTS (
    SELECT 1 FROM ranked_models 
    WHERE ranked_models.service_name = service_models.service_name 
    AND ranked_models.model_id = service_models.model_id
    AND ranked_models.new_position != service_models.position
);
