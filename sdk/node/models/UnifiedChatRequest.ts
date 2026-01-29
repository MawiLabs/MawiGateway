/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ChatMessage } from './ChatMessage';
import type { ChatParams } from './ChatParams';
import type { ResponseFormat } from './ResponseFormat';
import type { RoutingStrategy } from './RoutingStrategy';
export type UnifiedChatRequest = {
    service: string;
    messages: Array<ChatMessage>;
    params?: ChatParams;
    stream?: boolean;
    model?: string;
    routing_strategy?: RoutingStrategy;
    response_format?: ResponseFormat;
};

