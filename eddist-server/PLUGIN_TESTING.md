# Plugin System Testing Guide

This guide walks through testing the complete plugin system end-to-end.

## Prerequisites

- MySQL database with plugins table (migration applied)
- Redis server running
- eddist-server running (port 8080)
- eddist-admin running (port 8081)
- Admin credentials for accessing the admin panel

## Test Plan

### Phase 1: Setup and Database Migration

1. **Apply database migration**
```bash
cd /home/kain/workspace/eddist
sqlx migrate run --database-url "mysql://user:pass@localhost/eddist"
```

2. **Verify plugins table exists**
```bash
mysql -u user -p eddist -e "DESCRIBE plugins;"
```

Expected output:
```
+----------------+------------------+------+-----+
| Field          | Type             | Null | Key |
+----------------+------------------+------+-----+
| id             | bigint unsigned  | NO   | PRI |
| name           | varchar(255)     | NO   | UNI |
| description    | text             | YES  |     |
| script         | text             | NO   |     |
| enabled        | tinyint(1)       | NO   |     |
| hooks          | json             | NO   |     |
| http_whitelist | json             | YES  |     |
| created_at     | datetime(6)      | NO   |     |
| updated_at     | datetime(6)      | NO   |     |
+----------------+------------------+------+-----+
```

### Phase 2: Admin UI Testing

1. **Access Admin Panel**
   - Navigate to `http://localhost:8081/dashboard/plugins`
   - Login with admin credentials

2. **Verify Plugin List Page**
   - Should see empty state: "No plugins found"
   - Should see "Create New Plugin" button

3. **Create Test Plugin**
   - Click "Create New Plugin"
   - Fill in form:
     - Name: `test-plugin`
     - Description: `Test plugin for verification`
     - Select hooks: `before_post_thread`, `after_post_thread`
     - Script:
       ```lua
       function before_post_thread(data)
         local content = eddist.get_content(data)
         eddist.log("info", "TEST PLUGIN: Before thread - " .. content)
         return data
       end

       function after_post_thread(data)
         eddist.log("info", "TEST PLUGIN: After thread - success")
         return data
       end
       ```
   - Click "Create Plugin"

4. **Verify Plugin Created**
   - Should redirect to plugin list
   - Should see the test plugin with green "Enabled" badge
   - Should show 2 hooks: `before_post_thread`, `after_post_thread`

5. **Test Plugin Edit**
   - Click "Edit" on the test plugin
   - Modify description to `Updated test plugin`
   - Click "Update Plugin"
   - Verify changes are saved

6. **Test Plugin Toggle**
   - Click "Disable" button
   - Badge should change to gray "Disabled"
   - Click "Enable" button
   - Badge should change to green "Enabled"

### Phase 3: Functional Testing

1. **Start Server with Logs**
```bash
cd /home/kain/workspace/eddist/eddist-server
RUST_LOG=info cargo run 2>&1 | tee server.log
```

2. **Create Test Thread**

Using curl:
```bash
curl -X POST http://localhost:8080/test.cgi \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "bbs=test&FROM=TestUser&mail=sage&MESSAGE=Test+content&subject=Test+Thread&submit=新規スレッド作成"
```

Or use the web interface to create a thread.

3. **Verify Plugin Execution**

Check server logs for:
```
INFO TEST PLUGIN: Before thread - Test content
INFO Plugin 'test-plugin' executed successfully
INFO TEST PLUGIN: After thread - success
```

4. **Test Content Modification**

Create a spam filter plugin:
```lua
function before_post_thread(data)
  local content = eddist.get_content(data)
  if content:lower():find("spam") then
    eddist.set_content(data, "[FILTERED]")
  end
  return data
end
```

Post thread with "spam" in content:
```bash
curl -X POST http://localhost:8080/test.cgi \
  -d "bbs=test&FROM=TestUser&mail=sage&MESSAGE=Click+here+for+spam&subject=Test&submit=新規スレッド作成"
```

Verify thread content is "[FILTERED]" in database.

### Phase 4: Storage API Testing

1. **Create Post Counter Plugin**

Install from examples:
```bash
mysql -u user -p eddist < examples/plugins/04_post_counter.sql
```

2. **Enable Plugin**
   - Go to Admin UI
   - Enable "post-counter" plugin

3. **Create Multiple Threads**

Create 3-5 test threads.

4. **Verify Counter**

Check Redis:
```bash
redis-cli
> GET plugin:1:thread_count
"5"
```

Check logs for:
```
INFO Total threads created: 5
```

### Phase 5: Error Handling Testing

1. **Create Plugin with Syntax Error**

```lua
function before_post_thread(data
  -- Missing closing parenthesis
  return data
end
```

2. **Attempt to Create Thread**
   - Thread should still be created successfully
   - Plugin should fail gracefully
   - Check logs for Lua error message

3. **Create Plugin with Infinite Loop**

```lua
function before_post_thread(data)
  while true do
    -- Infinite loop
  end
  return data
end
```

4. **Attempt to Create Thread**
   - Should timeout after 5 seconds
   - Thread should still be created
   - Check logs for "Hook execution timed out"

### Phase 6: Multiple Plugin Chaining

1. **Create 3 Plugins**
   - Plugin A: Adds "[A]" prefix to content
   - Plugin B: Adds "[B]" prefix to content
   - Plugin C: Adds "[C]" prefix to content

```lua
-- Plugin A
function before_post_thread(data)
  local content = eddist.get_content(data)
  eddist.set_content(data, "[A] " .. content)
  return data
end

-- Plugin B (similar pattern)
-- Plugin C (similar pattern)
```

2. **Post Thread with "Test"**

3. **Verify Chain Execution**

Content should be: `[C] [B] [A] Test`

(Plugins execute in ID order)

### Phase 7: Response Testing

1. **Create Response Hook Plugin**

```lua
function before_post_response(data)
  eddist.log("info", "Response to thread: " .. tostring(data.thread_number))
  return data
end
```

2. **Post Response to Existing Thread**

```bash
curl -X POST http://localhost:8080/test.cgi \
  -d "bbs=test&key=1234567890&FROM=TestUser&mail=sage&MESSAGE=Test+response&submit=書き込む"
```

3. **Verify Logs**

Should see:
```
INFO Response to thread: 1234567890
```

## Expected Results Summary

- ✅ Plugins table created successfully
- ✅ Admin UI accessible and functional
- ✅ Can create, edit, enable/disable, delete plugins
- ✅ Monaco editor loads and works
- ✅ Plugins execute on thread creation
- ✅ Plugins execute on response creation
- ✅ Content modification works
- ✅ Storage API (Redis) works
- ✅ Logging works
- ✅ Plugins chain correctly (execute in order)
- ✅ Error handling prevents crashes
- ✅ Timeout protection works

## Troubleshooting

### Plugin Not Executing

1. Check if plugin is enabled
2. Check hook selection matches operation (thread vs response)
3. Verify Lua syntax
4. Check server logs for errors

### Monaco Editor Not Loading

1. Check browser console for errors
2. Verify `@monaco-editor/react` is installed:
   ```bash
   cd eddist-admin/client
   npm list @monaco-editor/react
   ```
3. Rebuild client if needed:
   ```bash
   npm install
   npm run build
   ```

### Storage Not Working

1. Verify Redis is running:
   ```bash
   redis-cli ping
   ```
2. Check Redis connection in server config
3. Check logs for Redis errors

### Admin UI Not Accessible

1. Check eddist-admin is running on port 8081
2. Verify authentication is configured
3. Check nginx/proxy configuration

## Performance Testing

### Load Test Plugin Execution

1. Create simple logging plugin
2. Use Apache Bench to create 100 threads:
   ```bash
   ab -n 100 -c 10 -p post_data.txt \
      -T "application/x-www-form-urlencoded" \
      http://localhost:8080/test.cgi
   ```
3. Monitor server logs for performance
4. Check for timeout warnings

### Expected Performance

- Plugin execution: < 10ms per plugin
- No timeouts under normal load
- Memory usage stays under 10MB per plugin

## Cleanup

After testing, clean up test data:

```bash
# Remove test plugins
mysql -u user -p eddist -e "DELETE FROM plugins WHERE name LIKE 'test%';"

# Clear Redis plugin data
redis-cli KEYS "plugin:*" | xargs redis-cli DEL

# Remove test threads (optional)
mysql -u user -p eddist -e "DELETE FROM threads WHERE board_key = 'test';"
```

## Next Steps

Once testing is complete:

1. Review logs for any issues
2. Document any bugs found
3. Create production plugins
4. Set up monitoring for plugin performance
5. Train admins on plugin management UI
