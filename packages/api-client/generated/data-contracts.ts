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

export enum VipStatus {
  Active = "active",
  Done = "done",
  Cancelled = "cancelled",
}

export enum VipKind {
  Vip = "vip",
  Unvip = "unvip",
}

/** Auto-generated discriminant enum variants */
export enum RuleTrigger {
  ChatMessage = "chat_message",
  RewardRedemption = "reward_redemption",
}

export enum QueueStatus {
  Pending = "Pending",
  Spinning = "Spinning",
  Completed = "Completed",
  Error = "Error",
  Cancelled = "Cancelled",
}

export enum OrderStatus {
  Pending = "pending",
  Completed = "completed",
  Cancelled = "cancelled",
}

export enum OrderSource {
  Donate = "donate",
  Points = "points",
  Roulette = "roulette",
  Other = "other",
}

export enum MovieKind {
  Movie = "movie",
  Series = "series",
  Anime = "anime",
  Youtube = "youtube",
}

export enum MessageMatcher {
  Contains = "contains",
  StartsWith = "starts_with",
  Equals = "equals",
  EndsWith = "ends_with",
}

export enum GameOrderKind {
  Stream = "stream",
  Playthrough = "playthrough",
}

/**
 * @format int32
 * @min 0
 */
export type ActionId = number;

export type ActionKind =
  | {
      type: "no_action";
    }
  | {
      type: "enqueue_roulette";
    }
  | {
      message_template: string;
      type: "chat_reply";
    };

export interface ActionResponse {
  created_at: string;
  enabled: boolean;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: ActionKind;
  name: string;
  updated_at: string;
}

export interface AddAdminRequest {
  display_name?: string | null;
  twitch_id: string;
}

export interface AdminGameOrderResponse {
  comment?: string | null;
  /** @format date */
  completed_at?: string | null;
  created_at: string;
  customer_name: string;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: GameOrderKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
  updated_at: string;
  /**
   * @format int32
   * @min 0
   */
  user_id?: number | null;
}

export interface AdminMovieOrderResponse {
  comment?: string | null;
  created_at: string;
  customer_name: string;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: MovieKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
  updated_at: string;
  /**
   * @format int32
   * @min 0
   */
  user_id?: number | null;
}

export interface AdminResponse {
  created_at: string;
  display_name?: string | null;
  is_root: boolean;
  twitch_id: string;
}

export interface AnonymousEnqueueRequest {
  name: string;
}

export interface CreateSessionRequest {
  ticket: string;
}

export interface CreateUserRequest {
  display_name: string;
}

export interface CreateVipRecordRequest {
  customer_name: string;
  /** @format date */
  end_date?: string | null;
  kind: VipKind;
  note?: string | null;
  /** @format date */
  roulette_date: string;
}

export interface EnqueueRequest {
  user_id: UserId;
  user_name: string;
}

export interface GameOrderResponse {
  comment?: string | null;
  completed_at?: string | null;
  customer_name: string;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: GameOrderKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
}

export interface ImportCountResponse {
  /**
   * @format int32
   * @min 0
   */
  imported: number;
  /**
   * @format int32
   * @min 0
   */
  skipped: number;
}

export interface ImportReportResponse {
  games: ImportCountResponse;
  movies: ImportCountResponse;
  spreadsheet_id: string;
  vip: ImportCountResponse;
}

export interface ImportRequest {
  spreadsheet_url: string;
}

export interface IngressCredentialsResponse {
  twitch: boolean;
  vk_video_live: boolean;
}

export interface LinkPlatformRequest {
  platform: string;
  platform_user_id: string;
  platform_username: string;
}

export interface MessageConditions {
  matcher: MessageMatcher;
  pattern?: string | null;
}

export interface MovieOrderResponse {
  comment?: string | null;
  customer_name: string;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: MovieKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
}

export interface NextResponse {
  entry: QueueEntryResponse;
  slot: RouletteSlot;
}

/**
 * @format int32
 * @min 0
 */
export type PlatformId = number;

export interface PlatformResponse {
  id: PlatformId;
  name: string;
}

export interface PresenceResponse {
  dock_connected: boolean;
  /** @min 0 */
  widget_count: number;
}

/**
 * @format int32
 * @min 0
 */
export type QueueEntryId = number;

export interface QueueEntryResponse {
  created_at: string;
  id: QueueEntryId;
  result_slot_id?: null | RouletteSlotId;
  slot_name?: string | null;
  status: QueueStatus;
  updated_at: string;
  user_id: UserId;
  user_name: string;
}

export interface QueueListResponse {
  entries: QueueEntryResponse[];
  next_cursor?: null | QueueEntryId;
}

export interface QueueStats {
  /**
   * @format int32
   * @min 0
   */
  cancelled: number;
  /**
   * @format int32
   * @min 0
   */
  completed: number;
  /**
   * @format int32
   * @min 0
   */
  error: number;
  /**
   * @format int32
   * @min 0
   */
  pending: number;
  /**
   * @format int32
   * @min 0
   */
  spinning: number;
}

/**
 * @format int32
 * @min 0
 */
export type RarityId = number;

export interface RarityResponse {
  color: string;
  display_name: string;
  id: RarityId;
  image: string;
  name: string;
}

export interface RewardConditions {
  platform?: null | PlatformId;
  reward_ids: string[];
}

export interface RewardResponse {
  /**
   * @format int64
   * @min 0
   */
  cost: number;
  id: string;
  is_enabled: boolean;
  is_paused: boolean;
  platform: PlatformId;
  title: string;
  used_in_rules: boolean;
}

export interface RouletteSlot {
  action: string;
  id: RouletteSlotId;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

/**
 * @format int32
 * @min 0
 */
export type RouletteSlotId = number;

export interface RouletteSlotResponse {
  action: string;
  id: RouletteSlotId;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

export type RuleConditions =
  | (MessageConditions & {
      trigger: "chat_message";
    })
  | (RewardConditions & {
      trigger: "reward_redemption";
    });

export interface RuleResponse {
  /**
   * @format int32
   * @min 0
   */
  action_id: number;
  conditions: RuleConditions;
  created_at: string;
  enabled: boolean;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  name: string;
  /** Auto-generated discriminant enum variants */
  trigger: RuleTrigger;
  updated_at: string;
}

export interface SessionResponse {
  expires_at: string;
  is_root: boolean;
  twitch_user_id: string;
  twitch_user_name?: string | null;
}

export interface SetStreamStatusRequest {
  online: boolean;
}

export interface SheetsStatusResponse {
  configured: boolean;
  last_synced_at?: string | null;
  spreadsheet_id: string;
}

export interface StreamStatusResponse {
  online: boolean;
}

export interface SyncReportResponse {
  /**
   * @format int32
   * @min 0
   */
  games: number;
  /**
   * @format int32
   * @min 0
   */
  movies: number;
  synced_at: string;
  /**
   * @format int32
   * @min 0
   */
  vip: number;
}

export interface TwitchAuthCallbackResponse {
  user_id: string;
  user_name?: string | null;
}

export interface TwitchAuthStartResponse {
  auth_url: string;
}

export interface TwitchLoginCallbackResponse {
  ticket: string;
  twitch_user_id: string;
  twitch_user_name?: string | null;
}

export interface TwitchLoginStartResponse {
  auth_url: string;
}

export interface TwitchUserResponse {
  display_name: string;
  id: string;
  login: string;
}

export interface UpdatePlatformRequest {
  platform_username: string;
}

export interface UpdateUserRequest {
  display_name: string;
}

export interface UpdateVipRecordRequest {
  customer_name: string;
  /** @format date */
  end_date: string;
  kind: VipKind;
  note?: string | null;
  /** @format date */
  roulette_date: string;
  status: VipStatus;
}

export interface UpsertActionRequest {
  enabled: boolean;
  kind: ActionKind;
  name: string;
}

export interface UpsertGameOrderRequest {
  comment?: string | null;
  /** @format date */
  completed_at?: string | null;
  customer_name: string;
  kind: GameOrderKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
}

export interface UpsertMovieOrderRequest {
  comment?: string | null;
  customer_name: string;
  kind: MovieKind;
  source: OrderSource;
  status: OrderStatus;
  title?: string | null;
}

export interface UpsertRarityRequest {
  color: string;
  display_name: string;
  image: string;
  name: string;
}

export interface UpsertRouletteSlotRequest {
  action: string;
  name: string;
  rarity_id: RarityId;
  /**
   * @format int64
   * @min 0
   */
  weight: number;
}

export interface UpsertRuleRequest {
  action_id: ActionId;
  conditions: RuleConditions;
  enabled: boolean;
  name: string;
  /** Auto-generated discriminant enum variants */
  trigger: RuleTrigger;
}

/**
 * @format int32
 * @min 0
 */
export type UserId = number;

/**
 * @format int32
 * @min 0
 */
export type UserPlatformId = number;

export interface UserPlatformResponse {
  id: UserPlatformId;
  platform: string;
  platform_user_id: string;
  platform_username: string;
}

export interface UserResponse {
  created_at: string;
  display_name: string;
  id: UserId;
  platforms: UserPlatformResponse[];
  updated_at: string;
}

export interface VipRecordResponse {
  created_at: string;
  customer_name: string;
  /** @format date */
  end_date: string;
  /**
   * @format int32
   * @min 0
   */
  id: number;
  kind: VipKind;
  note?: string | null;
  /** @format date */
  roulette_date: string;
  status: VipStatus;
  updated_at: string;
  /**
   * @format int32
   * @min 0
   */
  user_id?: number | null;
}

export interface VipRemindersResponse {
  awaiting_return: VipRecordResponse[];
  expiring: VipRecordResponse[];
}

export interface VkVideoLiveAuthCallbackResponse {
  user_id: string;
  user_name: string;
}

export interface VkVideoLiveAuthStartResponse {
  auth_url: string;
}

export interface WidgetAccessKeyResponse {
  widget_access_key: string;
}
