import { ApiErrorKind, normalizeApiError } from "#lib/internal/api-error";

function serverMessage(body: unknown): string | null {
	if (
		body !== null &&
		typeof body === "object" &&
		"error" in body &&
		typeof (body as { error: unknown }).error === "string"
	) {
		return (body as { error: string }).error;
	}
	return null;
}

/**
 * UI-facing error formatting: maps ApiError kinds to human-readable texts.
 * Lives outside internal/ on purpose — it contains product copy.
 */
export function describeApiError(err: unknown): string {
	const apiErr = normalizeApiError(err);
	const upstream = serverMessage(apiErr.body);
	switch (apiErr.kind) {
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
			return upstream ?? "Запрос отклонён сервером — проверь заполненные поля.";
		case ApiErrorKind.Server:
		case ApiErrorKind.HttpOther:
			return (
				upstream ?? `Ошибка на сервере (${apiErr.status ?? "нет статуса"}).`
			);
		case ApiErrorKind.Timeout:
			return "Сервер не ответил вовремя. Попробуй ещё раз.";
		case ApiErrorKind.Network:
			return "Нет связи с сервером. Проверь соединение.";
		default:
			return upstream ?? "Что-то пошло не так. Попробуй ещё раз.";
	}
}
