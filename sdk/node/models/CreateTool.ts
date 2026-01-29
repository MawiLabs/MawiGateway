/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Request to create a tool for an agentic service
 */
export type CreateTool = {
    name: string;
    description: string;
    tool_type: string;
    target_id: string;
    parameters_schema?: any;
    position: number;
};

