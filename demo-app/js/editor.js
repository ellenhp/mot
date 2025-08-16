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
  fontSize: "13pt",
  tabSize: 2,
  useSoftTabs: true
});
editor.session.setUseWorker(false)
editor.session.setMode("ace/mode/javascript");

editor.addEventListener('change', async (delta) => {
  if (ready) {
    var source = map.getSource('polyline')
    var ghost = map.getSource('polyline_ghost')
    if (source && ghost && (await source.getData())?.coordinates?.length > 0) {
      ghost.setData(await source.getData());
      source.setData({ type: "LineString", coordinates: [] });
    }
    routingWorker.postMessage({ costing_model: editor.getValue(), url: window.location.href });
  }
})

routingWorker.onmessage = (event) => {
  if (event.data === "ready") {
    routingWorker.postMessage({ costing_model: editor.getValue(), url: window.location.href });
    return;
  }
  var source = map.getSource('polyline')
  var ghost = map.getSource('polyline_ghost')
  if (source && ghost) {
    if (event.data) {
      source.setData(event.data);
      ghost.setData({ type: "LineString", coordinates: [] });
    } else {
      source.setData({ type: "LineString", coordinates: [] });
      ghost.setData({ type: "LineString", coordinates: [] });
    }
  } else {
    map.addSource('polyline', {
      type: 'geojson',
      data: event.data ? event.data : { type: "LineString", coordinates: [] }
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
    map.addSource('polyline_ghost', {
      type: 'geojson',
      data: { type: "LineString", coordinates: [] }
    });
    map.addLayer({
      "id": "polyline_ghost",
      "type": "line",
      "source": "polyline_ghost",
      "paint": {
        "line-color": "rgba(59, 94, 50, 1)",
        "line-opacity": 0.5,
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
      "id": "polyline_ghost_casing",
      "type": "line",
      "source": "polyline_ghost",
      "paint": {
        "line-color": "rgba(35, 173, 0, 1)",
        "line-opacity": 0.5,
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
    ready = true

  }
};

