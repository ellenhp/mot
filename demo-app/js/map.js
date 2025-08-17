import maplibregl from 'maplibre-gl';
import * as css from 'maplibre-gl/dist/maplibre-gl.css';
import { Protocol } from 'pmtiles';
import debounce from 'debounce';
import { routingWorker, editor } from './editor';


let protocol = new Protocol();
maplibregl.addProtocol("pmtiles", protocol.tile);

let style = require('../static/maplibre_style.json')

style.sources.intersections.url = "pmtiles://" + window.location.href + "seattle_intersections.pmtiles"
style.sources.roads.url = "pmtiles://" + window.location.href + "seattle_roads.pmtiles"

const map = new maplibregl.Map({
  container: 'map', // container id
  style: style,
  center: [-122.34766436214954, 47.661704892774935], // starting position [lng, lat]
  zoom: 13, // starting zoom
  minZoom: 11,
  maxBounds: [
    -122.709045, 47.253136, -121.813660, 47.924625
  ]
});

var start = null
var finish = null

map.on('load', function () {
  map.resize();
  start = new maplibregl.Marker({ draggable: true, color: '#3F3FCE' })
    .setLngLat([-122.3500866984068, 47.65145961351803])
    .addTo(map);
  finish = new maplibregl.Marker({ draggable: true, color: '#CE3F3F' })
    .setLngLat([-122.34230786420267, 47.67163303761944])
    .addTo(map);
  console.log(start)
  routingWorker.postMessage({ costing_model: editor.getValue(), url: window.location.href, start: start.getLngLat(), finish: finish.getLngLat() });

  start.on('drag', () => {
    routingWorker.postMessage({ costing_model: editor.getValue(), url: window.location.href, start: start.getLngLat(), finish: finish.getLngLat() });
  })
  finish.on('drag', () => {
    routingWorker.postMessage({ costing_model: editor.getValue(), url: window.location.href, start: start.getLngLat(), finish: finish.getLngLat() });
  })
});

const update_text = (e) => {
  var text = ""
  if (e && e.features) {
    for (let index = 0; index < e.features.length; index++) {
      const element = e.features[index];
      text += JSON.stringify(element.properties, null, 2) + "\n"
    }
  }
  document.getElementById('hover_properties').innerText = text
}

map.on('mousemove', 'roads', (e) => update_text(e));

map.on('mouseleave', 'roads', () => update_text());


export { map, start, finish };