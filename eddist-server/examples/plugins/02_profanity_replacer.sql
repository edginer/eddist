-- Profanity Replacer Plugin
-- Replaces profanity with asterisks

INSERT INTO plugins (id, name, description, script, enabled, hooks, permissions, created_at, updated_at)
VALUES (
  UUID_TO_BIN(UUID()),
  'profanity-replacer',
  'Replaces common profanity with asterisks to keep discussions civil',
  '-- Profanity Replacer Plugin
-- Replaces profanity with asterisks

local profanity_list = {
  "damn",
  "hell",
  "stupid",
  "idiot"
  -- Add more as needed
}

function replace_profanity(text)
  local result = text
  for _, word in ipairs(profanity_list) do
    -- Case insensitive replacement
    local pattern = word:gsub("(%a)", function(c)
      return "[" .. c:upper() .. c:lower() .. "]"
    end)
    local replacement = string.rep("*", #word)
    result = result:gsub(pattern, replacement)
  end
  return result
end

function before_post_thread(data)
  local content = eddist.get_content(data)
  local cleaned_content = replace_profanity(content)

  if content ~= cleaned_content then
    eddist.log("info", "Profanity filtered in thread")
    eddist.set_content(data, cleaned_content)
  end

  return data
end

function before_post_response(data)
  local content = eddist.get_content(data)
  local cleaned_content = replace_profanity(content)

  if content ~= cleaned_content then
    eddist.log("info", "Profanity filtered in response")
    eddist.set_content(data, cleaned_content)
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
