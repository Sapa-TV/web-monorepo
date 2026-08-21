/* eslint-disable */
/* tslint:disable */
// @ts-nocheck
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

import {
  AnonymousEnqueueRequest,
  CreateUserRequest,
  EnqueueRequest,
  LinkPlatformRequest,
  NextResponse,
  PlatformResponse,
  QueueEntryId,
  QueueEntryResponse,
  QueueListResponse,
  QueueStats,
  QueueStatus,
  RarityResponse,
  RouletteSlotResponse,
  SetStreamStatusRequest,
  StreamStatusResponse,
  UpdatePlatformRequest,
  UpdateUserRequest,
  UserId,
  UserResponse,
} from "./data-contracts";
import { ContentType, HttpClient, RequestParams } from "./http-client";

export class Wapi<
  SecurityDataType = unknown,
> extends HttpClient<SecurityDataType> {
  /**
   * No description
   *
   * @tags users
   * @name ListPlatforms
   * @request GET:/wapi/platforms
   */
  listPlatforms = (params: RequestParams = {}) =>
    this.request<PlatformResponse[], any>({
      path: `/wapi/platforms`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name List
   * @request GET:/wapi/queue
   */
  list = (
    status: null | QueueStatus,
    limit: number | null,
    cursor: null | QueueEntryId,
    params: RequestParams = {},
  ) =>
    this.request<QueueListResponse, any>({
      path: `/wapi/queue`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Enqueue
   * @request POST:/wapi/queue
   */
  enqueue = (data: EnqueueRequest, params: RequestParams = {}) =>
    this.request<QueueEntryResponse, any>({
      path: `/wapi/queue`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name EnqueueAnonymous
   * @request POST:/wapi/queue/anonymous
   */
  enqueueAnonymous = (
    data: AnonymousEnqueueRequest,
    params: RequestParams = {},
  ) =>
    this.request<QueueEntryResponse, any>({
      path: `/wapi/queue/anonymous`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name PeekNext
   * @request GET:/wapi/queue/next
   */
  peekNext = (params: RequestParams = {}) =>
    this.request<QueueEntryResponse, void>({
      path: `/wapi/queue/next`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name DequeueNext
   * @request POST:/wapi/queue/next
   */
  dequeueNext = (params: RequestParams = {}) =>
    this.request<NextResponse, void>({
      path: `/wapi/queue/next`,
      method: "POST",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Stats
   * @request GET:/wapi/queue/stats
   */
  stats = (params: RequestParams = {}) =>
    this.request<QueueStats, any>({
      path: `/wapi/queue/stats`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name GetById
   * @request GET:/wapi/queue/{id}
   */
  getById = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<QueueEntryResponse, void>({
      path: `/wapi/queue/${id}`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Cancel
   * @request POST:/wapi/queue/{id}/cancel
   */
  cancel = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/wapi/queue/${id}/cancel`,
      method: "POST",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Complete
   * @request POST:/wapi/queue/{id}/complete
   */
  complete = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/wapi/queue/${id}/complete`,
      method: "POST",
      ...params,
    });
  /**
   * No description
   *
   * @tags rarities
   * @name ListRarities
   * @request GET:/wapi/rarities
   */
  listRarities = (params: RequestParams = {}) =>
    this.request<RarityResponse[], any>({
      path: `/wapi/rarities`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags slots
   * @name ListSlots
   * @request GET:/wapi/slots
   */
  listSlots = (params: RequestParams = {}) =>
    this.request<RouletteSlotResponse[], any>({
      path: `/wapi/slots`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags stream
   * @name SetStreamStatus
   * @request POST:/wapi/stream/status
   */
  setStreamStatus = (
    data: SetStreamStatusRequest,
    params: RequestParams = {},
  ) =>
    this.request<StreamStatusResponse, any>({
      path: `/wapi/stream/status`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name FindUser
   * @request GET:/wapi/users
   */
  findUser = (
    platform: string,
    platformUserId: string,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/wapi/users`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name CreateUser
   * @request POST:/wapi/users
   */
  createUser = (data: CreateUserRequest, params: RequestParams = {}) =>
    this.request<UserResponse, any>({
      path: `/wapi/users`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name GetUser
   * @request GET:/wapi/users/{id}
   */
  getUser = (id: UserId, params: RequestParams = {}) =>
    this.request<UserResponse, void>({
      path: `/wapi/users/${id}`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name DeleteUser
   * @request DELETE:/wapi/users/{id}
   */
  deleteUser = (id: UserId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/wapi/users/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name UpdateUser
   * @request PATCH:/wapi/users/{id}
   */
  updateUser = (
    id: UserId,
    data: UpdateUserRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/wapi/users/${id}`,
      method: "PATCH",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name LinkPlatform
   * @request POST:/wapi/users/{id}/platforms
   */
  linkPlatform = (
    id: UserId,
    data: LinkPlatformRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/wapi/users/${id}/platforms`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name DeletePlatform
   * @request DELETE:/wapi/users/{id}/platforms/{platform}
   */
  deletePlatform = (id: UserId, platform: string, params: RequestParams = {}) =>
    this.request<UserResponse, void>({
      path: `/wapi/users/${id}/platforms/${platform}`,
      method: "DELETE",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name UpdatePlatformUsername
   * @request PATCH:/wapi/users/{id}/platforms/{platform}
   */
  updatePlatformUsername = (
    id: UserId,
    platform: string,
    data: UpdatePlatformRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/wapi/users/${id}/platforms/${platform}`,
      method: "PATCH",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
}
