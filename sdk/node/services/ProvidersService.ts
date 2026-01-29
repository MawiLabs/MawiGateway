/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CreateProvider } from '../models/CreateProvider';
import type { Provider } from '../models/Provider';
import type { ProviderResponse } from '../models/ProviderResponse';
import type { UpdateProvider } from '../models/UpdateProvider';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class ProvidersService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * List all providers
     * @returns ProviderResponse
     * @throws ApiError
     */
    public getProviders(): CancelablePromise<Array<ProviderResponse>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/providers',
        });
    }
    /**
     * Create model group
     * @param requestBody
     * @returns Provider
     * @throws ApiError
     */
    public postProviders(
        requestBody: CreateProvider,
    ): CancelablePromise<Provider> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/providers',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Get model group by ID
     * @param id
     * @returns ProviderResponse
     * @throws ApiError
     */
    public getProviders1(
        id: string,
    ): CancelablePromise<ProviderResponse> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/providers/{id}',
            path: {
                'id': id,
            },
        });
    }
    /**
     * Update model group
     * @param id
     * @param requestBody
     * @returns Provider
     * @throws ApiError
     */
    public putProviders(
        id: string,
        requestBody: UpdateProvider,
    ): CancelablePromise<Provider> {
        return this.httpRequest.request({
            method: 'PUT',
            url: '/providers/{id}',
            path: {
                'id': id,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Delete provider
     * @param id
     * @returns string
     * @throws ApiError
     */
    public deleteProviders(
        id: string,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/providers/{id}',
            path: {
                'id': id,
            },
        });
    }
}
