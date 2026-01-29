/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ChatChoice } from './ChatChoice';
import type { RoutingMetadata } from './RoutingMetadata';
import type { TokenUsage } from './TokenUsage';
export type UnifiedChatResponse = {
    id: string;
    object: string;
    created: number;
    model: string;
    choices: Array<ChatChoice>;
    usage?: TokenUsage;
    routing_metadata?: RoutingMetadata;
};

