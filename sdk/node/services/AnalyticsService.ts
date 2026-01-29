/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { AnalyticsSummary } from '../models/AnalyticsSummary';
import type { RequestLog } from '../models/RequestLog';
import type { TimeSeriesPoint } from '../models/TimeSeriesPoint';
import type { TopModel } from '../models/TopModel';
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';
export class AnalyticsService {
    constructor(public readonly httpRequest: BaseHttpRequest) {}
    /**
     * Get analytics overview with summary, time-series, and top models
     * @returns AnalyticsSummary
     * @throws ApiError
     */
    public getAnalyticsSummary(): CancelablePromise<AnalyticsSummary> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/analytics/summary',
        });
    }
    /**
     * Get time-series data for charts
     * @param range
     * @returns TimeSeriesPoint
     * @throws ApiError
     */
    public getAnalyticsTimeSeries(
        range?: string,
    ): CancelablePromise<Array<TimeSeriesPoint>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/analytics/time-series',
            query: {
                'range': range,
            },
        });
    }
    /**
     * Get top models with cost analysis
     * @returns TopModel
     * @throws ApiError
     */
    public getAnalyticsTopModels(): CancelablePromise<Array<TopModel>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/analytics/top-models',
        });
    }
    /**
     * Legacy getter for simple lists (paginated)
     * @param limit
     * @returns RequestLog
     * @throws ApiError
     */
    public getAnalyticsRequests(
        limit?: number,
    ): CancelablePromise<Array<RequestLog>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/analytics/requests',
            query: {
                'limit': limit,
            },
        });
    }
}
