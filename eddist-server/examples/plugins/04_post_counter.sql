-- Post Counter Plugin
-- Counts the total number of posts using Redis storage

INSERT INTO plugins (id, name, description, script, enabled, hooks, permissions, created_at, updated_at)
VALUES (
  UUID_TO_BIN(UUID()),
  'post-counter',
  'Maintains a counter of total threads and responses using Redis storage',
  '-- Post Counter Plugin
-- Demonstrates Redis storage API

function increment_counter(key)
  local current = eddist.storage.get(key)
  local count = 0

  if current then
    count = tonumber(current) or 0
  end

  count = count + 1
  eddist.storage.set(key, tostring(count), nil)  -- nil = no expiry

  return count
end

function after_post_thread(data)
  local count = increment_counter("thread_count")
  eddist.log("info", string.format("Total threads created: %d", count))
  return data
end

function after_post_response(data)
  local count = increment_counter("response_count")
  eddist.log("info", string.format("Total responses posted: %d", count))
  return data
end
',
  true,
  JSON_ARRAY('after_post_thread', 'after_post_response'),
  JSON_OBJECT(
    'allow_http', false,
    'http_whitelist', JSON_ARRAY(),
    'allow_storage', true
  ),
  NOW(6),
  NOW(6)
);
