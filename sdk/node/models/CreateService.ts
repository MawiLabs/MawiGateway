/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type CreateService = {
    name: string;
    service_type: string;
    description?: string;
    strategy?: string;
    guardrails: Array<string>;
    planner_model_id?: string;
    system_prompt?: string;
    max_iterations?: number;
};

