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
  ActionResponse,
  AddAdminRequest,
  AdminGameOrderResponse,
  AdminMovieOrderResponse,
  AdminResponse,
  CreateSessionRequest,
  CreateVipRecordRequest,
  GameOrderKind,
  GameOrderResponse,
  IngressCredentialsResponse,
  MovieKind,
  MovieOrderResponse,
  OrderSource,
  OrderStatus,
  PresenceResponse,
  RarityId,
  RarityResponse,
  RewardResponse,
  RouletteSlotId,
  RouletteSlotResponse,
  RuleResponse,
  SessionResponse,
  StreamStatusResponse,
  TwitchAuthCallbackResponse,
  TwitchAuthStartResponse,
  TwitchLoginCallbackResponse,
  TwitchLoginStartResponse,
  TwitchUserResponse,
  UpdateVipRecordRequest,
  UpsertActionRequest,
  UpsertGameOrderRequest,
  UpsertMovieOrderRequest,
  UpsertRarityRequest,
  UpsertRouletteSlotRequest,
  UpsertRuleRequest,
  VipRecordResponse,
  VipRemindersResponse,
  VkVideoLiveAuthCallbackResponse,
  VkVideoLiveAuthStartResponse,
  WidgetAccessKeyResponse,
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
   * @name ListActions
   * @request GET:/api/admin/actions
   */
  listActions = (params: RequestParams = {}) =>
    this.request<ActionResponse[], any>({
      path: `/api/admin/actions`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateAction
   * @request POST:/api/admin/actions
   */
  createAction = (data: UpsertActionRequest, params: RequestParams = {}) =>
    this.request<ActionResponse, any>({
      path: `/api/admin/actions`,
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
   * @name UpdateAction
   * @request PUT:/api/admin/actions/{id}
   */
  updateAction = (
    id: number,
    data: UpsertActionRequest,
    params: RequestParams = {},
  ) =>
    this.request<ActionResponse, void>({
      path: `/api/admin/actions/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteAction
   * @request DELETE:/api/admin/actions/{id}
   */
  deleteAction = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/actions/${id}`,
      method: "DELETE",
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
  revokeIngressCredentials = (
    query?: {
      platform?: any;
    },
    params: RequestParams = {},
  ) =>
    this.request<void, void>({
      path: `/api/admin/ingress/credentials`,
      method: "DELETE",
      query: query,
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateGameOrder
   * @request POST:/api/admin/orders/games
   */
  createGameOrder = (
    data: UpsertGameOrderRequest,
    params: RequestParams = {},
  ) =>
    this.request<AdminGameOrderResponse, any>({
      path: `/api/admin/orders/games`,
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
   * @name UpdateGameOrder
   * @request PUT:/api/admin/orders/games/{id}
   */
  updateGameOrder = (
    id: number,
    data: UpsertGameOrderRequest,
    params: RequestParams = {},
  ) =>
    this.request<AdminGameOrderResponse, void>({
      path: `/api/admin/orders/games/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteGameOrder
   * @request DELETE:/api/admin/orders/games/{id}
   */
  deleteGameOrder = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/orders/games/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateMovieOrder
   * @request POST:/api/admin/orders/movies
   */
  createMovieOrder = (
    data: UpsertMovieOrderRequest,
    params: RequestParams = {},
  ) =>
    this.request<AdminMovieOrderResponse, any>({
      path: `/api/admin/orders/movies`,
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
   * @name UpdateMovieOrder
   * @request PUT:/api/admin/orders/movies/{id}
   */
  updateMovieOrder = (
    id: number,
    data: UpsertMovieOrderRequest,
    params: RequestParams = {},
  ) =>
    this.request<AdminMovieOrderResponse, void>({
      path: `/api/admin/orders/movies/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteMovieOrder
   * @request DELETE:/api/admin/orders/movies/{id}
   */
  deleteMovieOrder = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/orders/movies/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListVipRecords
   * @request GET:/api/admin/orders/vip
   */
  listVipRecords = (params: RequestParams = {}) =>
    this.request<VipRecordResponse[], any>({
      path: `/api/admin/orders/vip`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateVipRecord
   * @request POST:/api/admin/orders/vip
   */
  createVipRecord = (
    data: CreateVipRecordRequest,
    params: RequestParams = {},
  ) =>
    this.request<VipRecordResponse, any>({
      path: `/api/admin/orders/vip`,
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
   * @name VipReminders
   * @request GET:/api/admin/orders/vip/reminders
   */
  vipReminders = (
    query?: {
      /**
       * @format int64
       * @min 0
       */
      days?: number | null;
    },
    params: RequestParams = {},
  ) =>
    this.request<VipRemindersResponse, any>({
      path: `/api/admin/orders/vip/reminders`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name UpdateVipRecord
   * @request PUT:/api/admin/orders/vip/{id}
   */
  updateVipRecord = (
    id: number,
    data: UpdateVipRecordRequest,
    params: RequestParams = {},
  ) =>
    this.request<VipRecordResponse, void>({
      path: `/api/admin/orders/vip/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteVipRecord
   * @request DELETE:/api/admin/orders/vip/{id}
   */
  deleteVipRecord = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/orders/vip/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetPresence
   * @request GET:/api/admin/presence
   */
  getPresence = (params: RequestParams = {}) =>
    this.request<PresenceResponse, any>({
      path: `/api/admin/presence`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListRewards
   * @request GET:/api/admin/rewards
   */
  listRewards = (params: RequestParams = {}) =>
    this.request<RewardResponse[], void>({
      path: `/api/admin/rewards`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListRarities
   * @request GET:/api/admin/roulette/rarities
   */
  listRarities = (params: RequestParams = {}) =>
    this.request<RarityResponse[], any>({
      path: `/api/admin/roulette/rarities`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateRarity
   * @request POST:/api/admin/roulette/rarities
   */
  createRarity = (data: UpsertRarityRequest, params: RequestParams = {}) =>
    this.request<RarityResponse, any>({
      path: `/api/admin/roulette/rarities`,
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
   * @name UpdateRarity
   * @request PUT:/api/admin/roulette/rarities/{id}
   */
  updateRarity = (
    id: RarityId,
    data: UpsertRarityRequest,
    params: RequestParams = {},
  ) =>
    this.request<RarityResponse, void>({
      path: `/api/admin/roulette/rarities/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteRarity
   * @request DELETE:/api/admin/roulette/rarities/{id}
   */
  deleteRarity = (id: RarityId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/roulette/rarities/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListSlots
   * @request GET:/api/admin/roulette/slots
   */
  listSlots = (params: RequestParams = {}) =>
    this.request<RouletteSlotResponse[], any>({
      path: `/api/admin/roulette/slots`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateSlot
   * @request POST:/api/admin/roulette/slots
   */
  createSlot = (data: UpsertRouletteSlotRequest, params: RequestParams = {}) =>
    this.request<RouletteSlotResponse, any>({
      path: `/api/admin/roulette/slots`,
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
   * @name UpdateSlot
   * @request PUT:/api/admin/roulette/slots/{id}
   */
  updateSlot = (
    id: RouletteSlotId,
    data: UpsertRouletteSlotRequest,
    params: RequestParams = {},
  ) =>
    this.request<RouletteSlotResponse, void>({
      path: `/api/admin/roulette/slots/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteSlot
   * @request DELETE:/api/admin/roulette/slots/{id}
   */
  deleteSlot = (id: RouletteSlotId, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/roulette/slots/${id}`,
      method: "DELETE",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name ListRules
   * @request GET:/api/admin/rules
   */
  listRules = (params: RequestParams = {}) =>
    this.request<RuleResponse[], any>({
      path: `/api/admin/rules`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name CreateRule
   * @request POST:/api/admin/rules
   */
  createRule = (data: UpsertRuleRequest, params: RequestParams = {}) =>
    this.request<RuleResponse, void>({
      path: `/api/admin/rules`,
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
   * @name UpdateRule
   * @request PUT:/api/admin/rules/{id}
   */
  updateRule = (
    id: number,
    data: UpsertRuleRequest,
    params: RequestParams = {},
  ) =>
    this.request<RuleResponse, void>({
      path: `/api/admin/rules/${id}`,
      method: "PUT",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name DeleteRule
   * @request DELETE:/api/admin/rules/{id}
   */
  deleteRule = (id: number, params: RequestParams = {}) =>
    this.request<void, void>({
      path: `/api/admin/rules/${id}`,
      method: "DELETE",
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
    query: {
      code: string;
      state: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchAuthCallbackResponse, void>({
      path: `/api/admin/twitch/auth/callback`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name FindTwitchUser
   * @request GET:/api/admin/twitch/users
   */
  findTwitchUser = (
    query: {
      login: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchUserResponse, void>({
      path: `/api/admin/twitch/users`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name StartVkVideoLiveAuth
   * @request GET:/api/admin/vk-video-live/auth
   */
  startVkVideoLiveAuth = (params: RequestParams = {}) =>
    this.request<VkVideoLiveAuthStartResponse, void>({
      path: `/api/admin/vk-video-live/auth`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name VkVideoLiveAuthCallback
   * @request GET:/api/admin/vk-video-live/auth/callback
   */
  vkVideoLiveAuthCallback = (
    query: {
      code: string;
      state: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<VkVideoLiveAuthCallbackResponse, void>({
      path: `/api/admin/vk-video-live/auth/callback`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name GetWidgetAccessKey
   * @request GET:/api/admin/widget-access-key
   */
  getWidgetAccessKey = (params: RequestParams = {}) =>
    this.request<WidgetAccessKeyResponse, any>({
      path: `/api/admin/widget-access-key`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags admin
   * @name RotateWidgetAccessKey
   * @request POST:/api/admin/widget-access-key
   */
  rotateWidgetAccessKey = (params: RequestParams = {}) =>
    this.request<WidgetAccessKeyResponse, any>({
      path: `/api/admin/widget-access-key`,
      method: "POST",
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
    query: {
      code: string;
      state: string;
    },
    params: RequestParams = {},
  ) =>
    this.request<TwitchLoginCallbackResponse, void>({
      path: `/api/auth/twitch/callback`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags system
   * @name Health
   * @request GET:/api/health
   */
  health = (params: RequestParams = {}) =>
    this.request<string, any>({
      path: `/api/health`,
      method: "GET",
      ...params,
    });
  /**
   * No description
   *
   * @tags orders
   * @name ListGameOrders
   * @request GET:/api/orders/games
   */
  listGameOrders = (
    query?: {
      status?: null | OrderStatus;
      kind?: null | GameOrderKind;
      source?: null | OrderSource;
      q?: string | null;
    },
    params: RequestParams = {},
  ) =>
    this.request<GameOrderResponse[], any>({
      path: `/api/orders/games`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * No description
   *
   * @tags orders
   * @name ListMovieOrders
   * @request GET:/api/orders/movies
   */
  listMovieOrders = (
    query?: {
      status?: null | OrderStatus;
      kind?: null | MovieKind;
      source?: null | OrderSource;
      q?: string | null;
    },
    params: RequestParams = {},
  ) =>
    this.request<MovieOrderResponse[], any>({
      path: `/api/orders/movies`,
      method: "GET",
      query: query,
      format: "json",
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
   * @tags system
   * @name Version
   * @request GET:/api/version
   */
  version = (params: RequestParams = {}) =>
    this.request<object, any>({
      path: `/api/version`,
      method: "GET",
      format: "json",
      ...params,
    });
}
