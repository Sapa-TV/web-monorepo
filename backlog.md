# Задачи

- [ ] Переделать на axum-extra (cookie) `let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");`
- [ ] Перенести все const в config, для части сделать default значения
- [ ] Разделить config - сейчас куча всего лежит в common
- [ ] перейти с .env ACCESS_KEY на генерируемый при старте и с root ендпойнотом для замены ключа
- [ ] посмотреть конфиги - зачем нужен refresh token в .env config? он будет храниться в db repo
- [ ] apps/backend/src/ingress/twitch_auth.rs - сейчас сохраняется в файл, надо переделать на inmemory repo.
      c трейтом и инмемори реализацией, позже это будет храниться в sqlite db
