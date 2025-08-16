import maplibregl from 'maplibre-gl';
import * as css from 'maplibre-gl/dist/maplibre-gl.css';
import { Protocol } from "pmtiles";


let protocol = new Protocol();
maplibregl.addProtocol("pmtiles", protocol.tile);

const map = new maplibregl.Map({
  container: 'map', // container id
  style: 'maplibre_style.json', // style URL
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


export default map;