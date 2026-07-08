-- wezterm-ah.lua — WezTerm integration for ah
--
-- Add to ~/.config/wezterm/wezterm.lua:
--   require 'wezterm-ah'
--
-- Usage: select text with mouse, then press SUPER+e
-- Shows result in a new pane

local wezterm = require 'wezterm'

local function ah_explain(window, pane)
  local sel = window:get_selection_text_for_pane(pane)
  local text = sel and sel:trim() or ''

  if text == '' then
    wezterm.log_info('ah: no selection, try selecting a word first')
    return
  end

  wezterm.run_child_process({
    'ah', 'explain', '--pipe',
  }, {
    stdin = text,
    pane = pane:split({ direction = 'Right', size = { Percent = 40 } }),
  })
end

wezterm.on('ah-explain', function(window, pane)
  ah_explain(window, pane)
end)

return {}
