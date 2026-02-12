---@meta

---@class ui
ui = {}

--- Open a file dialog to return a file path
---@param title string
---@return string|nil path
function ui.open_file(title) end

--- Open a folder dialog to return a folder path
---@param title string
---@return string|nil path
function ui.open_folder(title) end

--- Pops up an alert in the UI
---@param title string
function ui.alert(title) end

--- Asks for a string in a popup
---@param title string
---@return string value
function ui.ask_string(title) end

--- Asks for an integer in a popup
---@param title string
---@return integer value
function ui.ask_integer(title) end

--- Asks for an number (float/double) in a popup
---@param title string
---@return number value
function ui.ask_number(title) end


---@class steam
steam = {}

--- Returns the SteamID64 of the currently logged in steam user
---@param install_path string|nil
---@return integer|nil steam_id
function steam.get_current_user(install_path) end

--- Returns a list of steam ids that exist on this computer
---@param install_path string|nil
---@return integer[] users
function steam.get_users(install_path) end

--- Returns a list of save file paths associated with a game name or game id
--- Either app_id or game_name should be supplied
---@param id integer|Game Steam AppID for the game or the game's lua ID
---@return string[] paths Save file associated with the given game
function steam.save_file_paths(id) end

---@alias GameID "MHWILDS" | "DD2" | "PRAGMATA" | "MHST3"

---@class Game
---@field MHWILDS GameID Monster Hunter Wilds
---@field DD2 GameID Dragon's Dogma 2
---@field PRAGMATA GameID Pragmata
---@field MHST3 GameID Monster Hunter Stories 3
game = {
	MHWILDS = "MHWILDS",
	DD2 = "DD2",
	PRAGMATA = "PRAGMATA",
	MHST3 = "MHST3"
}

---@alias SaveNode Class | Array | integer | number | string | boolean | nil

---@class Class
---@field [string|integer] SaveNode
local Class = {}

---@class Array
---@field [integer] SaveNode
local Array = {}

---@class SaveFile
---@field [integer] Class|Array
local SaveFile = {}

--- Returns the number of top level object in the save file
---@operator len: number
function SaveFile:__len() end

--- Write the save file to the specified file
--- Save File are aware of what game they are loaded from so a GameID is not needed
---@param path string
---@param steamid integer
---@return string|nil error
function SaveFile:save(path, steamid) end


---@class fs
fs = {}

--- Loads a save file from the path
---@param path string
---@param steamid integer
---@param game GameID
---@return SaveFile
function fs.load_save(path, steamid, game) end
