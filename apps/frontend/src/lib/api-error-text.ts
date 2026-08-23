import { ApiError, ApiErrorKind } from "#lib/internal/api-error";

/**
 * UI-facing error formatting: maps ApiError kinds to human-readable texts.
 * Lives outside internal/ on purpose — it contains product copy.
 */
export function describeApiError(err: unknown): string {
	if (!(err instanceof ApiError)) {
		return err instanceof Error ? err.message : String(err);
	}
	switch (err.kind) {
		case ApiErrorKind.Unauthorized:
			return "Сессия истекла. Перелогинься.";
		case ApiErrorKind.Forbidden:
			return "Недостаточно прав для этого действия.";
		case ApiErrorKind.NotFound:
			return "Объект не найден — возможно, уже удалён.";
		case ApiErrorKind.Conflict:
			return "Такая запись уже существует.";
		case ApiErrorKind.RateLimited:
			return "Слишком много запросов. Подожди немного и попробуй снова.";
		case ApiErrorKind.BadRequest:
			return "Запрос отклонён сервером — проверь заполненные поля.";
		case ApiErrorKind.Server:
			return "Ошибка на сервере. Попробуй позже.";
		case ApiErrorKind.Timeout:
			return "Сервер не ответил вовремя. Попробуй ещё раз.";
		case ApiErrorKind.Network:
			return "Нет связи с сервером. Проверь соединение.";
		default:
			return "Что-то пошло не так. Попробуй ещё раз.";
	}
}
