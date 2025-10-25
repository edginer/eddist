-- Spam Filter Plugin
-- Filters content containing common spam keywords

INSERT INTO plugins (id, name, description, script, enabled, hooks, permissions, created_at, updated_at)
VALUES (
  UUID_TO_BIN(UUID()),
  'spam-filter',
  'Filters content containing spam keywords like "spam", "ads", "buy now"',
  '-- Spam Filter Plugin
-- Filters common spam keywords

local spam_keywords = {
  "spam",
  "click here",
  "buy now",
  "limited offer",
  "act now",
  "free money"
}

function before_post_thread(data)
  local content = eddist.get_content(data)
  local content_lower = content:lower()

  for _, keyword in ipairs(spam_keywords) do
    if content_lower:find(keyword, 1, true) then
      eddist.log("warn", "Spam keyword detected: " .. keyword)
      eddist.set_content(data, "[CONTENT FILTERED: Spam detected]")
      return data
    end
  end

  return data
end

function before_post_response(data)
  local content = eddist.get_content(data)
  local content_lower = content:lower()

  for _, keyword in ipairs(spam_keywords) do
    if content_lower:find(keyword, 1, true) then
      eddist.log("warn", "Spam keyword detected in response: " .. keyword)
      eddist.set_content(data, "[CONTENT FILTERED: Spam detected]")
      return data
    end
  end

  return data
end
',
  true,
  JSON_ARRAY('before_post_thread', 'before_post_response'),
  JSON_OBJECT(
    'allow_http', false,
    'http_whitelist', JSON_ARRAY(),
    'allow_storage', false
  ),
  NOW(6),
  NOW(6)
);
