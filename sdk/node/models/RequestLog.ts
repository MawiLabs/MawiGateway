/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type RequestLog = {
    id: string;
    service_name: string;
    model_id: string;
    provider_type: string;
    latency_ms: number;
    status: string;
    created_at: string;
    tokens_prompt?: number;
    tokens_completion?: number;
    tokens_total?: number;
    cost_usd?: number;
    error_message?: string;
    failover_count: number;
};

