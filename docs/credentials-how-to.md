# Как получать и хранить ключи (credentials how-to)

Шпаргалка на будущее: откуда брать каждый секрет, куда его класть локально и на VPS,
как перешифровать `.env.sops`.

## Общая схема

- Бэкенд читает конфиг в порядке: `.env` → `config.toml` → переменные окружения.
  Вложенные ключи задаются через `__` (например `TWITCH__CLIENT_ID`).
- Локально (dev): `.env.dev` (just подхватывает его через `set dotenv-filename`).
- Прод: на VPS лежит `.env.sops` (зашифрован, коммитится в репо). При деплое
  `deploy/scripts/deploy-backend.sh` расшифровывает его в `.env`
  (`sops -d ... > .env; chmod 600`), docker-compose подхватывает `.env` через `env_file`.
- **В репо коммитится только `.env.sops`.** `.env` и `.env.dev` — в .gitignore, не коммитить.

## SOPS / age: шифрование и расшифровка

- `.sops.yaml`: creation_rules для `^\.env$`, два age-ресивера
  (публичные ключи `age1...` уже в файле). Зашифровать может любой,
  **расшифровать — только владелец приватного age-ключа** (на VPS и у тебя локально).
  Приватный ключ: `~/.config/sops/age/keys.txt`, в репо НЕ кладём.
- Поменял `.env` → перешифруй и закоммить:
  ```sh
  just encrypt-env   # sops -e --input-type dotenv --output-type dotenv .env > .env.sops
  ```
- Посмотреть прод-секреты локально (нужен приватный ключ):
  ```sh
  just decrypt-env   # sops -d ... .env.sops > .env
  ```
- Если добавился новый ключ/ресивер — обновить `.sops.yaml` и перешифровать файл
  (`sops updatekeys .env.sops` или заново `just encrypt-env`).

## Twitch

Источник: <https://dev.twitch.tv/console/apps> → создать приложение.

| Переменная                         | Где взять                                                                                                                             |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `TWITCH__CLIENT_ID`                | Client ID приложения                                                                                                                  |
| `TWITCH__CLIENT_SECRET`            | Generate New Secret там же                                                                                                            |
| `TWITCH__BROADCASTER_ID`           | числовой id канала стримера (можно узнать через `api.twitch.tv/helix/users?login=...` или любой конвертер login→id)                   |
| `TWITCH__REDIRECT_URI`             | OAuth Redirect URL приложения, логин: `https://<домен>/login-callback/twitch` (dev: `http://localhost:5173/login-callback/twitch`)    |
| `TWITCH__CREDENTIALS_REDIRECT_URI` | второй Redirect URL, подключение канала: `https://<домен>/creds-callback/twitch` (dev: `http://localhost:5173/creds-callback/twitch`) |
| `TWITCH__CSRF_TTL_SECS`            | просто число, напр. 600                                                                                                               |

Оба redirect URI надо прописать в настройках приложения Twitch, иначе OAuth не пустит.

## VK Video Live

Источник: <https://id.vk.com/about/business/go> → создать приложение.

| Переменная                                | Где взять                                      |
| ----------------------------------------- | ---------------------------------------------- |
| `VK_VIDEO_LIVE__CLIENT_ID`                | ID приложения                                  |
| `VK_VIDEO_LIVE__CLIENT_SECRET`            | Защищённый ключ                                |
| `VK_VIDEO_LIVE__CHANNEL_URL`              | короткое имя канала VK Video Live              |
| `VK_VIDEO_LIVE__REDIRECT_URI`             | `https://<домен>/login-callback/vk-video-live` |
| `VK_VIDEO_LIVE__CREDENTIALS_REDIRECT_URI` | `https://<домен>/creds-callback/vk-video-live` |
| `VK_VIDEO_LIVE__CSRF_TTL_SECS`            | напр. 600                                      |

Redirect URI прописать в настройках приложения VK ID.

## Google Sheets (заказы игр/фильмов/VIP)

Делается один раз владельцем таблицы:

1. <https://console.cloud.google.com> → создать проект (или взять существующий).
2. **APIs & Services → Library** → включить **Google Sheets API**.
3. **IAM & Admin → Service Accounts → Create service account** (имя любое, напр. `sapa-sheets`).
   Роли на проект не нужны.
4. В созданном аккаунте: **Keys → Add key → Create new key → JSON** → скачать файл
   (вида `project-xxxx.json`). Это и есть секрет.
5. Открыть Google-таблицу → **Настройки доступа** → добавить email сервисного аккаунта
   (`xxx@project.iam.gserviceaccount.com`, виден в JSON-ключе в поле `client_email`)
   с правами **Редактор** (нужны и чтение — импорт, и запись — синк).

Куда класть ключ — **base64 в env** (едет через `.env.sops`,
никаких файлов и путей в контейнере):

1. Положить JSON в `secrets/google-sa.json` (папка в .gitignore) и выполнить:
   ```sh
   just google-key          # печатает готовую строку GOOGLE_SERVICE_ACCOUNT_KEY_BASE64=...
   just google-key путь\к\другому.json   # если файл лежит не там
   ```
2. Вставить одной строкой в `.env` / `.env.dev`:
   `GOOGLE_SERVICE_ACCOUNT_KEY_BASE64=eyJ0eXBlIjoic2VydmljZV9hY2NvdW50Ii...`
3. Дальше стандартно: `just encrypt-env`, коммит `.env.sops`, деплой.
   Сам JSON-файл после этого можно удалить — нигде на диске он не нужен.

| Переменная                          | Значение                                                                                                           |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `GOOGLE_SERVICE_ACCOUNT_KEY_BASE64` | JSON-ключ service account в base64, одна строка                                                                    |
| `GOOGLE_SPREADSHEET_ID`             | id таблицы из URL (`/d/<вот это>/edit`). Опционально: id также сохраняется в рантайм-конфиг при импорте из админки |

Текущая таблица: `1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk`.

Проверка: админка → Заказы → карточка Google Sheets должна показывать
«настроено» и принимать импорт. Если ключа нет/битый — бэкенд стартует без интеграции
(ошибка в логе), эндпоинты отвечают 400.

## Чеклист «добавил новый секрет»

1. Прописать в локальный `.env.dev` — проверить dev.
2. Прописать в прод `.env` (локальная копия прод-значений).
3. `just encrypt-env`, закоммитить `.env.sops` (и `.env.example` с пустым значением).
4. Деплой (`deploy/scripts/deploy-backend.sh` сам расшифрует `.env.sops` → `.env`).
