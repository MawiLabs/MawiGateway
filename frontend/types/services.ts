export enum ServiceType {
    POOL = 'POOL',
    AGENTIC = 'AGENTIC',
}

export enum PoolType {
    SINGLE_MODALITY = 'SINGLE_MODALITY',
    MULTI_MODALITY = 'MULTI_MODALITY',
}

export enum Modality {
    TEXT = 'text',
    IMAGE = 'image',
    AUDIO = 'audio',
    VIDEO = 'video',
}

export interface Service {
    name: string;
    description?: string;
    service_type: ServiceType | string;
    strategy: string;
    guardrails?: string;
    created_at?: number;

    // Pool specifics
    pool_type?: PoolType | string;
    input_modalities?: (Modality | string)[];
    output_modalities?: (Modality | string)[];

    // Agentic specifics
    planner_model_id?: string;
    system_prompt?: string;
    max_iterations?: number;
    user_id?: string;
}
