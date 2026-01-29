/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { AssignModel } from '../models/AssignModel';
import type { CreateService } from '../models/CreateService';
import type { CreateTool } from '../models/CreateTool';
import type { Service } from '../models/Service';
import type { UpdateService } from '../models/UpdateService';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class ServicesService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * @returns Service
     * @throws ApiError
     */
    public getServices(): CancelablePromise<Array<Service>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/services',
        });
    }
    /**
     * Create service
     * @param requestBody
     * @returns Service
     * @throws ApiError
     */
    public postServices(
        requestBody: CreateService,
    ): CancelablePromise<Service> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/services',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Get service by name
     * @param name
     * @returns Service
     * @throws ApiError
     */
    public getServices1(
        name: string,
    ): CancelablePromise<Service> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/services/{name}',
            path: {
                'name': name,
            },
        });
    }
    /**
     * Update service
     * @param name
     * @param requestBody
     * @returns Service
     * @throws ApiError
     */
    public putServices(
        name: string,
        requestBody: UpdateService,
    ): CancelablePromise<Service> {
        return this.httpRequest.request({
            method: 'PUT',
            url: '/services/{name}',
            path: {
                'name': name,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Delete service
     * @param name
     * @returns string
     * @throws ApiError
     */
    public deleteServices(
        name: string,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/services/{name}',
            path: {
                'name': name,
            },
        });
    }
    /**
     * Assign model to service (with modality validation and weight)
     * @param name
     * @param requestBody
     * @returns string
     * @throws ApiError
     */
    public postServicesModels(
        name: string,
        requestBody: AssignModel,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/services/{name}/models',
            path: {
                'name': name,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * Get models assigned to a service
     * @param name
     * @returns any
     * @throws ApiError
     */
    public getServicesModels(
        name: string,
    ): CancelablePromise<Array<any>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/services/{name}/models',
            path: {
                'name': name,
            },
        });
    }
    /**
     * Add a tool to an agentic service
     * @param name
     * @param requestBody
     * @returns string
     * @throws ApiError
     */
    public postServicesTools(
        name: string,
        requestBody: CreateTool,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/services/{name}/tools',
            path: {
                'name': name,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * List tools for an agentic service
     * @param name
     * @returns any
     * @throws ApiError
     */
    public getServicesTools(
        name: string,
    ): CancelablePromise<Array<any>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/services/{name}/tools',
            path: {
                'name': name,
            },
        });
    }
    /**
     * Delete a tool from an agentic service
     * @param name
     * @param toolId
     * @returns string
     * @throws ApiError
     */
    public deleteServicesTools(
        name: string,
        toolId: string,
    ): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'DELETE',
            url: '/services/{name}/tools/{tool_id}',
            path: {
                'name': name,
                'tool_id': toolId,
            },
        });
    }
}
