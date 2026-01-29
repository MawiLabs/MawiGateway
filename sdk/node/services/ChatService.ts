/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { UnifiedChatRequest } from '../models/UnifiedChatRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class ChatService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * Create chat completion
     * @param requestBody
     * @returns binary
     * @throws ApiError
     */
    public postChatCompletions(
        requestBody: UnifiedChatRequest,
    ): CancelablePromise<Blob> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/chat/completions',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
}
