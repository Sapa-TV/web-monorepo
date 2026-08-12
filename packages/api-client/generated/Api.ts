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
  AddAdminRequest,
  AdminResponse,
  AnonymousEnqueueRequest,
  CreateRarityRequest,
  CreateRouletteSlotRequest,
  CreateSessionRequest,
  CreateUserRequest,
  EnqueueRequest,
  IngressCredentialsResponse,
  LinkPlatformRequest,
  NextResponse,
  PakResponse,
  PlatformResponse,
  QueueEntryId,
  QueueEntryResponse,
  QueueListResponse,
  QueueStats,
  QueueStatus,
  RarityId,
  RarityResponse,
  RouletteSlotId,
  RouletteSlotResponse,
  SessionResponse,
  SetStreamStatusRequest,
  StreamStatusResponse,
  TwitchAuthCallbackResponse,
  TwitchAuthStartResponse,
  TwitchLoginCallbackResponse,
  TwitchLoginStartResponse,
  UpdatePlatformRequest,
  UpdateRarityRequest,
  UpdateRouletteSlotRequest,
  UpdateUserRequest,
  UserId,
  UserResponse,
} from "./data-contracts";
import { ContentType, HttpClient, RequestParams } from "./http-client";

export class Api<
  SecurityDataType = unknown,
> extends HttpClient<SecurityDataType> {
  /**
   * No description
   *
   * @tags admin
   * @name ListAdmins
   * @request GET:/api/admin
   */
  listAdmins = (params: RequestParams = {}) =>
    this.request<AdminResponse[], any>({
      path: `/api/admin`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name AddAdmin
   * @request POST:/api/admin
   */
  addAdmin = (data: AddAdminRequest, params: RequestParams = {}) =>
    this.request<AdminResponse, void>({
      path: `/api/admin`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetIngressCredentials
   * @request GET:/api/admin/ingress/credentials
   */
  getIngressCredentials = (params: RequestParams = {}) =>
    this.request<IngressCredentialsResponse, any>({
      path: `/api/admin/ingress/credentials`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RevokeIngressCredentials
   * @request DELETE:/api/admin/ingress/credentials
   */
  revokeIngressCredentials = (params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/ingress/credentials`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetAdminPak
   * @request GET:/api/admin/pak
   */
  getAdminPak = (params: RequestParams = {}) =>
    this.request<PakResponse, any>({
      path: `/api/admin/pak`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name StartTwitchAuth
   * @request GET:/api/admin/twitch/auth
   */
  startTwitchAuth = (params: RequestParams = {}) =>
    this.request<TwitchAuthStartResponse, void>({
      path: `/api/admin/twitch/auth`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name TwitchAuthCallback
   * @request GET:/api/admin/twitch/auth/callback
   */
  twitchAuthCallback = (
    code: string,
    state: string,
    params: RequestParams = {},
  ) =>
    this.request<TwitchAuthCallbackResponse, void>({
      path: `/api/admin/twitch/auth/callback`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RemoveAdmin
   * @request DELETE:/api/admin/{twitch_id}
   */
  removeAdmin = (twitchId: string, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/${twitchId}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name StartTwitchLogin
   * @request GET:/api/auth/twitch
   */
  startTwitchLogin = (params: RequestParams = {}) =>
    this.request<TwitchLoginStartResponse, void>({
      path: `/api/auth/twitch`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name TwitchLoginCallback
   * @request GET:/api/auth/twitch/callback
   */
  twitchLoginCallback = (
    code: string,
    state: string,
    params: RequestParams = {},
  ) =>
    this.request<TwitchLoginCallbackResponse, void>({
      path: `/api/auth/twitch/callback`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name ListPlatforms
   * @request GET:/api/platforms
   */
  listPlatforms = (params: RequestParams = {}) =>
    this.request<PlatformResponse[], any>({
      path: `/api/platforms`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name List
   * @request GET:/api/queue
   */
  list = (
    status: null | QueueStatus,
    limit: number | null,
    cursor: null | QueueEntryId,
    params: RequestParams = {},
  ) =>
    this.request<QueueListResponse, any>({
      path: `/api/queue`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Enqueue
   * @request POST:/api/queue
   */
  enqueue = (data: EnqueueRequest, params: RequestParams = {}) =>
    this.request<QueueEntryResponse, any>({
      path: `/api/queue`,
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
   * @request POST:/api/queue/anonymous
   */
  enqueueAnonymous = (
    data: AnonymousEnqueueRequest,
    params: RequestParams = {},
  ) =>
    this.request<QueueEntryResponse, any>({
      path: `/api/queue/anonymous`,
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
   * @request GET:/api/queue/next
   */
  peekNext = (params: RequestParams = {}) =>
    this.request<QueueEntryResponse, void>({
      path: `/api/queue/next`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name DequeueNext
   * @request POST:/api/queue/next
   */
  dequeueNext = (params: RequestParams = {}) =>
    this.request<NextResponse, void>({
      path: `/api/queue/next`,
      method: "POST",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Stats
   * @request GET:/api/queue/stats
   */
  stats = (params: RequestParams = {}) =>
    this.request<QueueStats, any>({
      path: `/api/queue/stats`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name GetById
   * @request GET:/api/queue/{id}
   */
  getById = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<QueueEntryResponse, void>({
      path: `/api/queue/${id}`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Cancel
   * @request POST:/api/queue/{id}/cancel
   */
  cancel = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/queue/${id}/cancel`,
      method: "POST",
      ...params,
    });
  /**
   * No description
   *
   * @tags queue
   * @name Complete
   * @request POST:/api/queue/{id}/complete
   */
  complete = (id: QueueEntryId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/queue/${id}/complete`,
      method: "POST",
      ...params,
    });
  /**
   * No description
   *
   * @tags rarities
   * @name ListRarities
   * @request GET:/api/rarities
   */
  listRarities = (params: RequestParams = {}) =>
    this.request<RarityResponse[], any>({
      path: `/api/rarities`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags rarities
   * @name CreateRarity
   * @request POST:/api/rarities
   */
  createRarity = (data: CreateRarityRequest, params: RequestParams = {}) =>
    this.request<RarityResponse, any>({
      path: `/api/rarities`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags rarities
   * @name UpdateRarity
   * @request PUT:/api/rarities/{id}
   */
  updateRarity = (
    id: RarityId,
    data: UpdateRarityRequest,
    params: RequestParams = {},
  ) =>
    this.request<RarityResponse, void>({
      path: `/api/rarities/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags rarities
   * @name DeleteRarity
   * @request DELETE:/api/rarities/{id}
   */
  deleteRarity = (id: RarityId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/rarities/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name CreateSession
   * @request POST:/api/sessions
   */
  createSession = (data: CreateSessionRequest, params: RequestParams = {}) =>
    this.request<SessionResponse, void>({
      path: `/api/sessions`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name GetMe
   * @request GET:/api/sessions/me
   */
  getMe = (params: RequestParams = {}) =>
    this.request<SessionResponse, void>({
      path: `/api/sessions/me`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags auth
   * @name Logout
   * @request DELETE:/api/sessions/me
   */
  logout = (params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/sessions/me`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags slots
   * @name ListSlots
   * @request GET:/api/slots
   */
  listSlots = (params: RequestParams = {}) =>
    this.request<RouletteSlotResponse[], any>({
      path: `/api/slots`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags slots
   * @name CreateSlot
   * @request POST:/api/slots
   */
  createSlot = (data: CreateRouletteSlotRequest, params: RequestParams = {}) =>
    this.request<RouletteSlotResponse, void>({
      path: `/api/slots`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags slots
   * @name UpdateSlot
   * @request PUT:/api/slots/{id}
   */
  updateSlot = (
    id: RouletteSlotId,
    data: UpdateRouletteSlotRequest,
    params: RequestParams = {},
  ) =>
    this.request<RouletteSlotResponse, void>({
      path: `/api/slots/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags slots
   * @name DeleteSlot
   * @request DELETE:/api/slots/{id}
   */
  deleteSlot = (id: RouletteSlotId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/slots/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags stream
   * @name GetStreamStatus
   * @request GET:/api/stream/status
   */
  getStreamStatus = (params: RequestParams = {}) =>
    this.request<StreamStatusResponse, any>({
      path: `/api/stream/status`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags stream
   * @name SetStreamStatus
   * @request POST:/api/stream/status
   */
  setStreamStatus = (
    data: SetStreamStatusRequest,
    params: RequestParams = {},
  ) =>
    this.request<StreamStatusResponse, any>({
      path: `/api/stream/status`,
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
   * @request GET:/api/users
   */
  findUser = (
    platform: string,
    platformUserId: string,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/api/users`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name CreateUser
   * @request POST:/api/users
   */
  createUser = (data: CreateUserRequest, params: RequestParams = {}) =>
    this.request<UserResponse, any>({
      path: `/api/users`,
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
   * @request GET:/api/users/{id}
   */
  getUser = (id: UserId, params: RequestParams = {}) =>
    this.request<UserResponse, void>({
      path: `/api/users/${id}`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name DeleteUser
   * @request DELETE:/api/users/{id}
   */
  deleteUser = (id: UserId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/users/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name UpdateUser
   * @request PATCH:/api/users/{id}
   */
  updateUser = (
    id: UserId,
    data: UpdateUserRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/api/users/${id}`,
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
   * @request POST:/api/users/{id}/platforms
   */
  linkPlatform = (
    id: UserId,
    data: LinkPlatformRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/api/users/${id}/platforms`,
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
   * @request DELETE:/api/users/{id}/platforms/{platform}
   */
  deletePlatform = (id: UserId, platform: string, params: RequestParams = {}) =>
    this.request<UserResponse, void>({
      path: `/api/users/${id}/platforms/${platform}`,
      method: "DELETE",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags users
   * @name UpdatePlatformUsername
   * @request PATCH:/api/users/{id}/platforms/{platform}
   */
  updatePlatformUsername = (
    id: UserId,
    platform: string,
    data: UpdatePlatformRequest,
    params: RequestParams = {},
  ) =>
    this.request<UserResponse, void>({
      path: `/api/users/${id}/platforms/${platform}`,
      method: "PATCH",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
}
