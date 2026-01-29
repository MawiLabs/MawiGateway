/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { LoginReq } from '../models/LoginReq';
import type { RegisterReq } from '../models/RegisterReq';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class AuthService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * @param requestBody
     * @returns any
     * @throws ApiError
     */
    public postAuthRegister(
        requestBody: RegisterReq,
    ): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/auth/register',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * @param requestBody
     * @returns any
     * @throws ApiError
     */
    public postAuthLogin(
        requestBody: LoginReq,
    ): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/auth/login',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * @returns string
     * @throws ApiError
     */
    public postAuthLogout(): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/auth/logout',
        });
    }
    /**
     * @returns any
     * @throws ApiError
     */
    public getAuthMe(): CancelablePromise<any> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/auth/me',
        });
    }
}
