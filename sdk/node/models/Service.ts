/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Modality } from './Modality';
import type { PoolType } from './PoolType';
import type { ServiceType } from './ServiceType';
export type Service = {
    name: string;
    service_type: ServiceType;
    description?: string;
    strategy: string;
    guardrails?: string;
    created_at?: number;
    pool_type?: PoolType;
    input_modalities: Array<Modality>;
    output_modalities: Array<Modality>;
    planner_model_id?: string;
    system_prompt?: string;
    max_iterations?: number;
    user_id?: string;
};

