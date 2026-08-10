# Задачи

- [ ] Переделать на axum-extra (cookie) `let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");`
- [ ] Перенести все const в config, для части сделать default значения
- [ ] Разделить config - сейчас куча всего лежит в common
- [ ] apps/backend/src/ingress/twitch_auth.rs - сейчас сохраняется в файл, надо переделать на inmemory repo.
      c трейтом и инмемори реализацией, позже это будет храниться в sqlite db
