/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKeyInfo } from '../models/ApiKeyInfo';
import type { CreateApiKeyRequest } from '../models/CreateApiKeyRequest';
import type { CreateApiKeyResponse } from '../models/CreateApiKeyResponse';
import type { ModelInfo } from '../models/ModelInfo';
import type { ProviderInfo } from '../models/ProviderInfo';
import type { QuotaStatusResponse } from '../models/QuotaStatusResponse';
import type { Service } from '../models/Service';
import type { UserProfileResponse } from '../models/UserProfileResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class UserService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * @returns UserProfileResponse
     * @throws ApiError
     */
    public getUserMe(): CancelablePromise<UserProfileResponse> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/me',
        });
    }
    /**
     * @returns QuotaStatusResponse
     * @throws ApiError
     */
    public getUserQuota(): CancelablePromise<QuotaStatusResponse> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/quota',
        });
    }
    /**
     * @returns ProviderInfo
     * @throws ApiError
     */
    public getUserProviders(): CancelablePromise<Array<ProviderInfo>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/providers',
        });
    }
    /**
     * @returns Service
     * @throws ApiError
     */
    public getUserServices(): CancelablePromise<Array<Service>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/services',
        });
    }
    /**
     * @returns ModelInfo
     * @throws ApiError
     */
    public getUserModels(): CancelablePromise<Array<ModelInfo>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/models',
        });
    }
    /**
     * @returns any
     * @throws ApiError
     */
    public getUserLogs(): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/logs',
        });
    }
    /**
     * @returns any
     * @throws ApiError
     */
    public getUserAnalytics(): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/analytics',
        });
    }
    /**
     * @param modelId
     * @returns any
     * @throws ApiError
     */
    public postUserModelsHealth(
        modelId: string,
    ): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/user/models/{model_id}/health',
            path: {
                'model_id': modelId,
            },
        });
    }
    /**
     * @returns ApiKeyInfo
     * @throws ApiError
     */
    public getUserApiKeys(): CancelablePromise<Array<ApiKeyInfo>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/user/api-keys',
        });
    }
    /**
     * @param requestBody
     * @returns CreateApiKeyResponse
     * @throws ApiError
     */
    public postUserApiKeys(
        requestBody: CreateApiKeyRequest,
    ): CancelablePromise<CreateApiKeyResponse> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/user/api-keys',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * @param id
     * @returns boolean
     * @throws ApiError
     */
    public deleteUserApiKeys(
        id: string,
    ): CancelablePromise<boolean> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/user/api-keys/{id}',
            path: {
                'id': id,
            },
        });
    }
}
