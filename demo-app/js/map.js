import maplibregl from 'maplibre-gl';
import * as css from 'maplibre-gl/dist/maplibre-gl.css';
import { Protocol } from "pmtiles";
import debounce from 'debounce';


let protocol = new Protocol();
maplibregl.addProtocol("pmtiles", protocol.tile);

let style = require('../static/maplibre_style.json')

style.sources.intersections.url = "pmtiles://" + window.location.href + "seattle_intersections.pmtiles"
style.sources.roads.url = "pmtiles://" + window.location.href + "seattle_roads.pmtiles"

const map = new maplibregl.Map({
  container: 'map', // container id
  style: style,
  center: [-122.34983362124629, 47.658177754997155], // starting position [lng, lat]
  zoom: 14, // starting zoom
  minZoom: 11,
  maxBounds: [
    -122.709045, 47.253136, -121.813660, 47.924625
  ]
});

map.on('load', function () {
  map.resize();
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


export default map;