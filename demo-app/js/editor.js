import * as ace from 'ace-builds';

require('ace-builds/src-noconflict/theme-twilight')
require('ace-builds/src-noconflict/mode-javascript')
require('ace-builds/src-noconflict/ace')
// require('ace-builds/src-noconflict/mode-javascript')

var editor = ace.edit("editor");
editor.setTheme("ace/theme/twilight");
editor.setOptions({
  fontSize: "13pt"
});
editor.session.setUseWorker(false)
editor.session.setMode("ace/mode/javascript");


