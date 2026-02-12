# Array

## [integer]


```lua
boolean|string|number|Array|Class...(+1)
```


---

# Class

## [string|integer]


```lua
boolean|string|number|Array|Class...(+1)
```


---

# LuaLS


---

# SaveFile

## [integer]


```lua
Array|Class
```

## __len


```lua
(method) SaveFile:__len()
```

## save


```lua
(method) SaveFile:save(path: string)
  -> error: string|nil
```

 Write the save file to the specified file


---

# SaveNode


---

# fs

## load_save


```lua
function fs.load_save(path: string, steamid: integer)
  -> SaveFile
```

 Loads a save file from the path


---

# fs


```lua
fs
```


---

# fs.load_save


```lua
function fs.load_save(path: string, steamid: integer)
  -> SaveFile
```


---

# nil


---

# steam

## get_current_user


```lua
function steam.get_current_user(install_path: string|nil)
  -> steam_id: integer|nil
```

 Returns the SteamID64 of the currently logged in steam user

## get_users


```lua
function steam.get_users(install_path: string|nil)
  -> users: integer[]
```

 Returns a list of steam ids that exist on this computer

## save_file_paths


```lua
function steam.save_file_paths(id: string|integer)
  -> paths: string[]
```

 Returns a list of save file paths associated with a game name or game id
 Either app_id or game_name should be supplied

@*param* `id` — Steam AppID for the game or the game's full name

@*return* `paths` — Save file associated with the given game


---

# steam


```lua
steam
```


---

# steam.get_current_user


```lua
function steam.get_current_user(install_path: string|nil)
  -> steam_id: integer|nil
```


---

# steam.get_users


```lua
function steam.get_users(install_path: string|nil)
  -> users: integer[]
```


---

# steam.save_file_paths


```lua
function steam.save_file_paths(id: string|integer)
  -> paths: string[]
```


---

# ui


```lua
ui
```


---

# ui

## alert


```lua
function ui.alert(msg: string)
```

 Pops up an alert in the UI

## ask_number


```lua
function ui.ask_number(msg: string)
  -> value: integer
```

 Asks for a number in a popup

## ask_string


```lua
function ui.ask_string(msg: string)
  -> value: string
```

 Asks for a string in a popup

## open_file


```lua
function ui.open_file(msg: string)
  -> path: string|nil
```

 Open a file dialog to return a file path

## open_folder


```lua
function ui.open_folder(msg: string)
  -> path: string|nil
```

 Open a folder dialog to return a folder path


---

# ui.alert


```lua
function ui.alert(msg: string)
```


---

# ui.ask_number


```lua
function ui.ask_number(msg: string)
  -> value: integer
```


---

# ui.ask_string


```lua
function ui.ask_string(msg: string)
  -> value: string
```


---

# ui.open_file


```lua
function ui.open_file(msg: string)
  -> path: string|nil
```


---

# ui.open_folder


```lua
function ui.open_folder(msg: string)
  -> path: string|nil
```