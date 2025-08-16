import * as ace from 'ace-builds';
import map from './map'

require('ace-builds/src-noconflict/theme-twilight')
require('ace-builds/src-noconflict/mode-javascript')
require('ace-builds/src-noconflict/ace')


const routingWorker = new Worker(new URL('./routing.js', import.meta.url));
var ready = false;

var editor = ace.edit("editor");
editor.setTheme("ace/theme/twilight");
editor.setOptions({
  fontSize: "13pt"
});
editor.session.setUseWorker(false)
editor.session.setMode("ace/mode/javascript");

editor.addEventListener('change', (delta) => {
  if (ready) {
    routingWorker.postMessage(editor.getValue());
  }
})

routingWorker.onmessage = (event) => {
  if (event.data == null) {
    routingWorker.postMessage(editor.getValue());
    ready = true
    return;
  }
  var source = map.getSource('polyline')
  if (source) {
    source.setData(event.data);
  } else {
    map.addSource('polyline', {
      type: 'geojson',
      data: event.data
    });
    map.addLayer({
      "id": "polyline_casing",
      "type": "line",
      "source": "polyline",
      "paint": {
        "line-color": "rgba(59, 94, 50, 1)",
        "line-width": {
          "stops": [
            [
              10,
              6
            ],
            [
              14,
              12
            ]
          ]
        }
      }
    });
    map.addLayer({
      "id": "polyline",
      "type": "line",
      "source": "polyline",
      "paint": {
        "line-color": "rgba(35, 173, 0, 1)",
        "line-width": {
          "stops": [
            [
              10,
              3
            ],
            [
              14,
              6
            ]
          ]
        }
      }
    });

  }
};

