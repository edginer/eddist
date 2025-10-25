-- Content Logger Plugin
-- Logs information about all posts

INSERT INTO plugins (id, name, description, script, enabled, hooks, permissions, created_at, updated_at)
VALUES (
  UUID_TO_BIN(UUID()),
  'content-logger',
  'Logs statistics about threads and responses for analytics',
  '-- Content Logger Plugin
-- Logs information about content creation

function count_characters(text)
  return #text
end

function count_words(text)
  local count = 0
  for word in text:gmatch("%S+") do
    count = count + 1
  end
  return count
end

function before_post_thread(data)
  local content = eddist.get_content(data)
  local char_count = count_characters(content)
  local word_count = count_words(content)

  eddist.log("info", string.format(
    "New thread - Characters: %d, Words: %d",
    char_count,
    word_count
  ))

  return data
end

function after_post_thread(data)
  eddist.log("info", "Thread successfully created")
  return data
end

function before_post_response(data)
  local content = eddist.get_content(data)
  local char_count = count_characters(content)
  local word_count = count_words(content)

  eddist.log("info", string.format(
    "New response - Characters: %d, Words: %d",
    char_count,
    word_count
  ))

  return data
end

function after_post_response(data)
  eddist.log("info", "Response successfully posted")
  return data
end
',
  true,
  JSON_ARRAY('before_post_thread', 'after_post_thread', 'before_post_response', 'after_post_response'),
  JSON_OBJECT(
    'allow_http', false,
    'http_whitelist', JSON_ARRAY(),
    'allow_storage', false
  ),
  NOW(6),
  NOW(6)
);
