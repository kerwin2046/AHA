" vim-ah: Explain word under cursor or yanked text with ah
" Install: add to ~/.vim/pack/plugins/start/vim-ah/ or use plugin manager
"
" Commands:
"   :AH              Explain word under cursor
"   :AH <word>       Explain a specific word
"
" Mappings:
"   <leader>kt       Explain word under cursor
"
" Options:
"   g:ah_binary      Path to ah binary (default: 'ah')
"   g:ah_yank_auto   Auto-explain on yank in Vim (default: 1)
"
" Note: Vim's normal y/y does NOT sync to system clipboard by default.
"       For ah daemon (Ctrl+C workflow), add to vimrc:
"         set clipboard=unnamedplus

if exists('g:loaded_vim_ah')
  finish
endif
let g:loaded_vim_ah = 1

if !exists('g:ah_binary')
  let g:ah_binary = 'ah'
endif
if !exists('g:ah_yank_auto')
  let g:ah_yank_auto = 1
endif

function! s:parse_result(result) abort
  if v:shell_error
    return {'error': a:result}
  endif

  try
    return json_decode(a:result)
  catch
    return {'error': 'failed to parse response'}
  endtry
endfunction

function! s:result_lines(data) abort
  let lines = []
  if has_key(a:data, 'translation') && a:data['translation'] !=# ''
    call add(lines, '翻译: ' . a:data['translation'])
    call add(lines, '')
  endif
  if has_key(a:data, 'full_name') && a:data['full_name'] !=# ''
    call add(lines, '全称: ' . a:data['full_name'])
    call add(lines, '')
  endif
  if has_key(a:data, 'explanation') && a:data['explanation'] !=# ''
    call add(lines, '解释: ' . a:data['explanation'])
    call add(lines, '')
  endif
  if has_key(a:data, 'usage') && a:data['usage'] !=# ''
    call add(lines, '用法:')
    for usage_line in split(a:data['usage'], "\n")
      call add(lines, '  ' . usage_line)
    endfor
  endif
  return lines
endfunction

function! s:show_float(title, lines) abort
  if empty(a:lines)
    echomsg 'ah: no result'
    return
  endif

  if has('nvim')
    let buf = nvim_create_buf(v:false, v:true)
    call nvim_buf_set_lines(buf, 0, -1, v:false, a:lines)

    let width = min([80, &columns - 4])
    let height = min([len(a:lines) + 2, &lines - 4])
    let row = (&lines - height) / 2
    let col = (&columns - width) / 2

    let opts = {
          \ 'relative': 'editor',
          \ 'width': width,
          \ 'height': height,
          \ 'row': row,
          \ 'col': col,
          \ 'style': 'minimal',
          \ 'border': 'rounded',
          \ 'title': ' ah: ' . a:title . ' ',
          \ }
    let win = nvim_open_win(buf, v:true, opts)

    nnoremap <buffer> <silent> q :close<CR>
    nnoremap <buffer> <silent> <Esc> :close<CR>
  else
    new
    setlocal buftype=nofile bufhidden=wipe noswapfile
    call setline(1, a:lines)
    execute 'file ah:' . a:title
    nnoremap <buffer> <silent> q :close<CR>
  endif
endfunction

function! s:ah_explain(word) abort
  let cmd = g:ah_binary . ' explain --json ' . shellescape(a:word)
  let data = s:parse_result(system(cmd))

  if has_key(data, 'error')
    echohl ErrorMsg
    echomsg 'ah: ' . data['error']
    echohl None
    return
  endif

  call s:show_float(a:word, s:result_lines(data))
endfunction

function! s:explain_yank() abort
  if !get(g:, 'ah_yank_auto', 1)
    return
  endif

  let text = substitute(getreg('"'), '\n$', '', '')
  let text = trim(text)
  if strlen(text) < 2
    return
  endif

  " Simple debounce for duplicate yank events.
  if exists('s:ah_yank_last') && s:ah_yank_last ==# text && localtime() - get(s:, 'ah_yank_time', 0) < 2
    return
  endif
  let s:ah_yank_last = text
  let s:ah_yank_time = localtime()

  let cmd = g:ah_binary . ' explain --pipe --json'
  let data = s:parse_result(system(cmd, text))

  if has_key(data, 'error')
    echohl ErrorMsg
    echomsg 'ah: ' . data['error']
    echohl None
    return
  endif

  let title = strlen(text) > 40 ? text[:37] . '...' : text
  call s:show_float(title, s:result_lines(data))
endfunction

command! -nargs=? AH call s:ah_explain(<q-args> !=# '' ? <q-args> : expand('<cword>'))

if !hasmapto(':AH<CR>', 'n')
  nmap <unique> <leader>kt :AH<CR>
endif

" Auto-explain when yanking in Vim (y/yy/visual-y).
if has('textyankpost')
  autocmd TextYankPost * if v:event.operator ==# 'y' | call s:explain_yank() | endif
endif
