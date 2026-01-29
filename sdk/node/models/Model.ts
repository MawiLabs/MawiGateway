/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type Model = {
    id: string;
    name: string;
    provider: string;
    modality: string;
    description?: string;
    cost_per_1k_tokens?: number;
    cost_per_1k_input_tokens?: number;
    cost_per_1k_output_tokens?: number;
    tier: string;
    avg_latency_ms: number;
    avg_ttft_ms: number;
    max_tps: number;
    api_endpoint?: string;
    api_version?: string;
    api_key?: string;
    created_at?: number;
    tier_required: string;
    worker_type: string;
    created_by?: string;
    user_id?: string;
};

