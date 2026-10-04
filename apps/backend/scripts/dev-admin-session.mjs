// Seeds a local dev admin + long-lived session into data/server.db and prints the cookie.
// Usage: just dev-admin
import { DatabaseSync } from "node:sqlite";

const TWITCH_ID = "1";
const TOKEN = "devtoken123";

const db = new DatabaseSync("data/server.db");
db.prepare(
	"INSERT OR REPLACE INTO admins (twitch_id, display_name, is_root, created_at) VALUES (?,?,1,datetime('now'))",
).run(TWITCH_ID, "dev-admin");
db.prepare(
	"INSERT OR REPLACE INTO sessions (token, twitch_user_id, twitch_user_name, created_at, expires_at) " +
		"VALUES (?,?,?,strftime('%Y-%m-%dT%H:%M:%f+00:00','now'),strftime('%Y-%m-%dT%H:%M:%f+00:00','now','+30 days'))",
).run(TOKEN, TWITCH_ID, "dev-admin");
db.close();
console.log(`admin session seeded; cookie: sapa_session=${TOKEN}`);
