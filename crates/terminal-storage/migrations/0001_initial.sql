CREATE TABLE host_groups (
 id TEXT PRIMARY KEY NOT NULL, parent_id TEXT REFERENCES host_groups(id) ON DELETE RESTRICT,
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128), sort_order INTEGER NOT NULL DEFAULT 0,
 revision INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE credentials (
 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('password','privateKey')),
 label TEXT NOT NULL, owner_profile_id TEXT,
 store_backend TEXT NOT NULL DEFAULT 'os', store_account TEXT NOT NULL UNIQUE,
 status TEXT NOT NULL CHECK(status IN ('pending','ready','deleting')),
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
 CHECK(owner_profile_id IS NULL OR kind='password')
);
CREATE TABLE key_files (
 id TEXT PRIMARY KEY NOT NULL, path TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL,
 content_fingerprint TEXT NOT NULL,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE connection_profiles (
 id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL DEFAULT 'ssh' CHECK(kind IN ('ssh','local','serial')),
 name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
 group_id TEXT REFERENCES host_groups(id) ON DELETE SET NULL,
 favorite INTEGER NOT NULL CHECK(favorite IN (0,1)), config_version INTEGER NOT NULL DEFAULT 1,
 config_json TEXT NOT NULL DEFAULT '{}', revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE ssh_profiles (
 profile_id TEXT PRIMARY KEY NOT NULL REFERENCES connection_profiles(id) ON DELETE CASCADE,
 address TEXT NOT NULL, canonical_host TEXT NOT NULL, port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535), username TEXT NOT NULL
);
CREATE TABLE profile_auth_steps (
 id TEXT PRIMARY KEY NOT NULL, profile_id TEXT NOT NULL REFERENCES connection_profiles(id) ON DELETE CASCADE,
 sort_order INTEGER NOT NULL, method TEXT NOT NULL CHECK(method IN ('password','privateKey')),
 credential_id TEXT REFERENCES credentials(id) ON DELETE RESTRICT,
 CHECK(method='password' OR credential_id IS NOT NULL),
 UNIQUE(profile_id,sort_order)
);
CREATE INDEX credentials_owner ON credentials(owner_profile_id);
CREATE TABLE tags (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL UNIQUE);
CREATE TABLE profile_tags (
 profile_id TEXT NOT NULL REFERENCES connection_profiles(id) ON DELETE CASCADE,
 tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE, PRIMARY KEY(profile_id,tag_id)
);
CREATE TABLE known_hosts (
 id TEXT PRIMARY KEY NOT NULL, canonical_host TEXT NOT NULL, port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535),
 key_algorithm TEXT NOT NULL, public_key BLOB NOT NULL, fingerprint_sha256 TEXT NOT NULL,
 trusted_at INTEGER NOT NULL, last_seen_at INTEGER,
 UNIQUE(canonical_host,port,key_algorithm,public_key)
);
CREATE INDEX known_hosts_endpoint ON known_hosts(canonical_host,port);
CREATE INDEX profiles_group ON connection_profiles(group_id,name);
CREATE INDEX ssh_search_address ON ssh_profiles(canonical_host,port);
CREATE TABLE settings (key TEXT PRIMARY KEY NOT NULL, value_json TEXT NOT NULL, value_version INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE session_history (
 id TEXT PRIMARY KEY NOT NULL, profile_id TEXT REFERENCES connection_profiles(id) ON DELETE SET NULL,
 kind TEXT NOT NULL DEFAULT 'ssh', display_name_snapshot TEXT NOT NULL, endpoint_snapshot TEXT,
 started_at INTEGER NOT NULL, ended_at INTEGER, end_reason TEXT, error_code TEXT
);
