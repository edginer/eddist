# Eddist Plugin System

The Eddist plugin system allows you to extend and customize the behavior of your BBS using Lua scripts. Plugins can intercept and modify thread and response creation, log analytics, filter content, and more.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Hook Points](#hook-points)
- [Eddist Lua API](#eddist-lua-api)
- [Creating Plugins](#creating-plugins)
- [Managing Plugins](#managing-plugins)
- [Examples](#examples)
- [Best Practices](#best-practices)
- [Security](#security)

## Overview

Plugins are Lua 5.4 scripts that execute at specific hook points during thread and response creation. Each plugin:

- Runs in a sandboxed environment with limited memory (10MB) and dynamic execution timeout
  - **1 second** timeout for plugins with HTTP permission enabled
  - **500ms** timeout for plugins without HTTP permission
- Can modify content before it's saved to the database
- Can execute side effects after successful operations
- Has access to Redis storage for persistent data (if storage permission is granted)
- Can make HTTP requests (if HTTP permission is granted with whitelist configuration)

## Architecture

```
User POST Request
      ↓
BeforePost Hook (modify data)
      ↓
Service Layer (validation, DB insert)
      ↓
AfterPost Hook (side effects)
      ↓
Response to User
```

### Plugin Execution

- Plugins execute sequentially in order of their database ID
- If a plugin errors, it logs the error and continues with the next plugin
- Plugins cannot prevent the operation from completing (use validation in app code for that)
- Modified data from one plugin is passed to the next plugin

## Hook Points

### before_post_thread

Executed before a new thread is created.

**Input Data:**
```lua
{
  "board_key": "string",
  "title": "string",
  "name": "string",
  "mail": "string",
  "body": "string"
}
```

**Can Modify:** title, name, mail, body

**Example:**
```lua
function before_post_thread(data)
  local content = eddist.get_content(data)
  eddist.set_content(data, "[MODIFIED] " .. content)
  return data
end
```

### after_post_thread

Executed after a thread is successfully created.

**Input Data:**
```lua
{
  "board_key": "string",
  "title": "string",
  "name": "string",
  "mail": "string",
  "body": "string",
  "success": true
}
```

**Example:**
```lua
function after_post_thread(data)
  eddist.log("info", "Thread created on board: " .. data.board_key)
  return data
end
```

### before_post_response

Executed before a response is posted to a thread.

**Input Data:**
```lua
{
  "board_key": "string",
  "thread_number": number,
  "name": "string",
  "mail": "string",
  "body": "string"
}
```

**Can Modify:** name, mail, body

**Example:**
```lua
function before_post_response(data)
  local content = eddist.get_content(data)
  if #content > 1000 then
    eddist.set_content(data, content:sub(1, 1000) .. "...")
  end
  return data
end
```

### after_post_response

Executed after a response is successfully posted.

**Input Data:**
```lua
{
  "board_key": "string",
  "thread_number": number,
  "name": "string",
  "mail": "string",
  "body": "string",
  "success": true,
  "res_order": number | null
}
```

**Example:**
```lua
function after_post_response(data)
  eddist.log("info", "Response #" .. tostring(data.res_order) .. " posted")
  return data
end
```

## Eddist Lua API

### Content Manipulation

#### `eddist.get_content(data) -> string`

Gets the content/body text from the data object.

```lua
local content = eddist.get_content(data)
```

#### `eddist.set_content(data, new_content)`

Sets new content/body text in the data object.

```lua
eddist.set_content(data, "New content here")
```

#### `eddist.get_author_id(data) -> string | nil`

Gets the author ID if available in the data.

```lua
local author_id = eddist.get_author_id(data)
if author_id then
  eddist.log("info", "Author: " .. author_id)
end
```

### Storage API

Plugin-specific Redis storage with namespace isolation.

**⚠️ Requires Permission:** `allow_storage: true`

When storage permission is not granted, the `eddist.storage` API will not be available.

#### `eddist.storage.get(key) -> string | nil`

Retrieves a value from storage.

```lua
local value = eddist.storage.get("my_key")
if value then
  eddist.log("info", "Found value: " .. value)
end
```

#### `eddist.storage.set(key, value, ttl)`

Stores a value with optional TTL (time-to-live) in seconds.

```lua
-- Store permanently
eddist.storage.set("counter", "42", nil)

-- Store for 1 hour
eddist.storage.set("temp_data", "value", 3600)
```

#### `eddist.storage.delete(key) -> boolean`

Deletes a key from storage.

```lua
local deleted = eddist.storage.delete("old_key")
```

#### `eddist.storage.exists(key) -> boolean`

Checks if a key exists in storage.

```lua
if eddist.storage.exists("counter") then
  eddist.log("info", "Counter exists")
end
```

### Logging

#### `eddist.log(level, message)`

Logs a message to the server logs.

**Levels:** "info", "warn", "error"

```lua
eddist.log("info", "Processing post")
eddist.log("warn", "Suspicious content detected")
eddist.log("error", "Failed to process")
```

### HTTP API

**⚠️ Requires Permission:** `allow_http: true` with configured `http_whitelist`

HTTP access must be explicitly enabled and configured with a whitelist. When HTTP permission is not granted, the `eddist.http` API will not be available.

**Note:** Plugins with HTTP permission enabled have a longer execution timeout (1 second vs 500ms).

#### `eddist.http.get(url) -> {status: number, body: string}`

Makes a GET request to a whitelisted URL.

```lua
local response = eddist.http.get("https://api.example.com/data")
if response.status == 200 then
  eddist.log("info", "Response: " .. response.body)
end
```

#### `eddist.http.post(url, body) -> {status: number, body: string}`

Makes a POST request with a JSON body.

```lua
local response = eddist.http.post(
  "https://api.example.com/webhook",
  '{"event":"new_post"}'
)
```

## Creating Plugins

### Via Admin UI

1. Navigate to **Dashboard > Plugins**
2. Click **Create New Plugin**
3. Fill in:
   - **Name**: Unique identifier (e.g., "spam-filter")
   - **Description**: What the plugin does
   - **Hooks**: Select which hooks to implement
   - **Script**: Write your Lua code
4. Click **Create Plugin**

### Via SQL

```sql
INSERT INTO plugins (id, name, description, script, enabled, hooks, permissions, created_at, updated_at)
VALUES (
  UUID_TO_BIN(UUID()),
  'my-plugin',
  'Plugin description',
  'function before_post_thread(data) return data end',
  true,
  JSON_ARRAY('before_post_thread'),
  JSON_OBJECT(
    'allow_http', false,
    'http_whitelist', JSON_ARRAY(),
    'allow_storage', false
  ),
  NOW(6),
  NOW(6)
);
```

### Via API

```bash
curl -X POST http://localhost:8081/api/plugins \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-plugin",
    "description": "Plugin description",
    "script": "function before_post_thread(data) return data end",
    "hooks": ["before_post_thread"],
    "permissions": {
      "allow_http": false,
      "http_whitelist": [],
      "allow_storage": false
    }
  }'
```

## Managing Plugins

### List All Plugins

**UI:** Dashboard > Plugins

**API:**
```bash
GET /api/plugins
```

### Get Plugin

**API:**
```bash
GET /api/plugins/{id}
```

### Update Plugin

**UI:** Dashboard > Plugins > Edit

**API:**
```bash
PUT /api/plugins/{id}
{
  "name": "updated-plugin",
  "description": "Updated description",
  "script": "...",
  "hooks": ["before_post_thread"],
  "permissions": {
    "allow_http": false,
    "http_whitelist": [],
    "allow_storage": false
  },
  "enabled": true
}
```

### Enable/Disable Plugin

**UI:** Dashboard > Plugins > Enable/Disable button

**API:**
```bash
PUT /api/plugins/{id}/toggle
{"enabled": false}
```

### Delete Plugin

**UI:** Dashboard > Plugins > Delete button

**API:**
```bash
DELETE /api/plugins/{id}
```

## Examples

### Example 1: Spam Filter

```lua
local spam_keywords = {"spam", "click here", "buy now"}

function before_post_thread(data)
  local content = eddist.get_content(data)
  local content_lower = content:lower()

  for _, keyword in ipairs(spam_keywords) do
    if content_lower:find(keyword, 1, true) then
      eddist.log("warn", "Spam detected: " .. keyword)
      eddist.set_content(data, "[CONTENT FILTERED]")
      return data
    end
  end

  return data
end
```

### Example 2: Post Counter with Storage

```lua
function after_post_thread(data)
  local current = eddist.storage.get("thread_count") or "0"
  local count = tonumber(current) + 1
  eddist.storage.set("thread_count", tostring(count), nil)
  eddist.log("info", "Total threads: " .. count)
  return data
end
```

### Example 3: Content Analytics

```lua
function before_post_response(data)
  local content = eddist.get_content(data)
  local word_count = select(2, content:gsub("%S+", ""))

  eddist.log("info", string.format(
    "Response stats - Chars: %d, Words: %d",
    #content, word_count
  ))

  return data
end
```

## Best Practices

1. **Keep Scripts Small**: Focus on one specific task per plugin
2. **Handle Nil Values**: Always check if values exist before using them
3. **Use Logging Wisely**: Don't log on every execution, only important events
4. **Return Data**: Always return the data object from hook functions
5. **Test Thoroughly**: Test with various inputs before enabling in production
6. **Monitor Performance**: Check logs for timeout warnings
7. **Version Control**: Keep plugin scripts in version control
8. **Document Intent**: Add comments explaining what your plugin does

## Security

### Sandbox Restrictions

Plugins run in a restricted Lua environment:

- **No File I/O**: `io` library is disabled
- **No OS Access**: `os` library is disabled
- **No External Loading**: `require`, `load`, `loadfile` are disabled
- **No Debugging**: `debug` library is disabled
- **Memory Limit**: 10MB per plugin
- **Execution Timeout**:
  - 1 second for plugins with `allow_http: true`
  - 500ms for plugins with `allow_http: false`
- **Storage Access**: Only available if `allow_storage: true`
- **HTTP Access**: Only available if `allow_http: true` with configured whitelist

### Available Lua Standard Libraries

- `string` - String manipulation
- `table` - Table operations
- `math` - Mathematical functions
- `utf8` - UTF-8 string handling

### Permissions

All plugin permissions are configured in a single `permissions` JSON object:

```json
{
  "allow_http": false,
  "http_whitelist": [],
  "allow_storage": false
}
```

#### Permission Fields

- **`allow_http`**: Enable HTTP request capability (default: `false`)
- **`http_whitelist`**: Array of allowed URL patterns with methods (only used when `allow_http` is `true`)
- **`allow_storage`**: Enable Redis storage access (default: `false`)

#### HTTP Whitelist

When `allow_http` is enabled, you must configure the whitelist:

```json
{
  "allow_http": true,
  "http_whitelist": [
    {
      "url_pattern": "https://api.example.com/webhook",
      "methods": ["POST"]
    },
    {
      "url_pattern": "https://api.example.com/*",
      "methods": ["GET", "POST"]
    }
  ],
  "allow_storage": false
}
```

**Wildcard Support:**
- Use `*` for wildcard matching in URL patterns
- `https://api.example.com/*` matches `https://api.example.com/any/path`
- `https://*/api/endpoint` matches any domain with that path
- Patterns are matched in order

**Notes:**
- If `allow_http` is `false`, the `http_whitelist` is ignored
- If `allow_http` is `true` but `http_whitelist` is empty, no HTTP requests will succeed
- HTTP-enabled plugins have a longer timeout (1 second vs 500ms)

### Error Handling

- Plugin errors are logged but don't crash the server
- Failed plugins are skipped; execution continues with next plugin
- Original data is preserved if plugin fails

## Troubleshooting

### Plugin Not Executing

1. Check if plugin is **enabled**
2. Verify correct **hook selection**
3. Check server logs for errors
4. Verify Lua syntax (no syntax errors)

### Script Timeouts

- Reduce complexity of script
- Avoid infinite loops
- Optimize string operations
- Consider breaking into multiple smaller plugins

### Storage Issues

- Check Redis connection
- Verify key naming (namespaced per plugin)
- Check for Redis memory limits
- Monitor storage quota (if implemented)

## Migration from Legacy Systems

If migrating from other BBS systems:

1. Audit existing customizations
2. Create equivalent Lua plugins
3. Test in development environment
4. Enable plugins one at a time in production
5. Monitor logs for issues

## Support

For issues, questions, or contributions:
- GitHub: https://github.com/edginer/eddist
- Documentation: Check openapi.json for API specs
