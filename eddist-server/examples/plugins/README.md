# Example Plugins

This directory contains example plugins demonstrating various features of the Eddist plugin system.

## Installation

To install these example plugins, run the SQL files against your MySQL database:

```bash
# Install all examples
for file in *.sql; do
  mysql -u your_user -p your_database < "$file"
done

# Or install individually
mysql -u your_user -p your_database < 01_spam_filter.sql
```

Alternatively, copy the Lua scripts and create plugins via the Admin UI at `/dashboard/plugins`.

## Available Examples

### 1. Spam Filter (`01_spam_filter.sql`)

**Purpose:** Filters content containing common spam keywords

**Hooks:** `before_post_thread`, `before_post_response`

**Features:**
- Detects spam keywords like "spam", "click here", "buy now"
- Replaces entire content with "[CONTENT FILTERED: Spam detected]"
- Logs warnings when spam is detected

**Use Case:** Prevent spam and unwanted advertising on your BBS

---

### 2. Profanity Replacer (`02_profanity_replacer.sql`)

**Purpose:** Replaces profanity with asterisks

**Hooks:** `before_post_thread`, `before_post_response`

**Features:**
- Case-insensitive profanity detection
- Replaces each profane word with asterisks (same length)
- Logs when filtering occurs

**Use Case:** Maintain a family-friendly community

---

### 3. Content Logger (`03_content_logger.sql`)

**Purpose:** Logs statistics about all posts for analytics

**Hooks:** `before_post_thread`, `after_post_thread`, `before_post_response`, `after_post_response`

**Features:**
- Counts characters and words in each post
- Logs analytics before and after posting
- Demonstrates using all four hook points

**Use Case:** Analytics and monitoring post activity

---

### 4. Post Counter (`04_post_counter.sql`)

**Purpose:** Maintains counters using Redis storage

**Hooks:** `after_post_thread`, `after_post_response`

**Features:**
- Uses Redis storage API
- Maintains persistent counters across restarts
- Increments on each successful post
- Logs total counts

**Use Case:** Track total activity metrics

## Customization

Feel free to modify these examples to suit your needs:

1. **Spam Filter**: Add more keywords to the `spam_keywords` table
2. **Profanity Replacer**: Customize the word list in `profanity_list`
3. **Content Logger**: Add more statistics or export to external systems
4. **Post Counter**: Track more granular metrics (per board, per user, etc.)

## Creating Your Own Plugins

Use these examples as templates. Key patterns:

### Data Modification Pattern
```lua
function before_post_thread(data)
  local content = eddist.get_content(data)
  -- Modify content...
  eddist.set_content(data, modified_content)
  return data
end
```

### Logging Pattern
```lua
function after_post_response(data)
  eddist.log("info", "Event occurred")
  return data
end
```

### Storage Pattern
```lua
function after_post_thread(data)
  local value = eddist.storage.get("key") or "0"
  eddist.storage.set("key", new_value, ttl)
  return data
end
```

## Testing

1. Install a plugin
2. Enable it in the Admin UI (`/dashboard/plugins`)
3. Create a test thread or response
4. Check server logs for plugin output:
   ```bash
   tail -f /var/log/eddist/server.log | grep "Plugin"
   ```

## Troubleshooting

- **Plugin not executing**: Check if it's enabled in the Admin UI
- **Syntax errors**: Validate Lua syntax before saving
- **Timeout errors**: Simplify complex operations
- **Storage issues**: Verify Redis is running and accessible

## More Examples

See the main documentation at `/eddist-server/PLUGINS.md` for:
- Complete API reference
- Best practices
- Advanced patterns
- Security guidelines
