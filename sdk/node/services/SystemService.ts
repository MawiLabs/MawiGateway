/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { TopologyResponse } from '../models/TopologyResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class SystemService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * Get full system topology (Providers, Services, Models) - filtered by authenticated user
     * @returns TopologyResponse
     * @throws ApiError
     */
    public getTopology(): CancelablePromise<TopologyResponse> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/topology',
        });
    }
}
