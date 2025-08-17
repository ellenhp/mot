import { toGeoJSON } from '@mapbox/polyline';
import debounce from 'debounce';
import { PMTiles } from 'pmtiles';

import { main_js, clear, ingest_tile, search } from '../pkg/index.js';

main_js();

const lon2tile = (lon, zoom) => (Math.floor((lon + 180) / 360 * Math.pow(2, zoom)));
const lat2tile = (lat, zoom) => (Math.floor((1 - Math.log(Math.tan(lat * Math.PI / 180) + 1 / Math.cos(lat * Math.PI / 180)) / Math.PI) / 2 * Math.pow(2, zoom)));


const process_event = debounce(async (event) => {
  clear();

  const costing_model = eval(event.data.costing_model);
  const start = event.data.start;
  const finish = event.data.finish;
  let pm_intersections = new PMTiles(event.data.url + "seattle_intersections.pmtiles")
  let pm_roads = new PMTiles(event.data.url + "seattle_roads.pmtiles")

  let start_tile_x = lon2tile(start.lng, 14);
  let start_tile_y = lat2tile(start.lat, 14);
  let finish_tile_x = lon2tile(finish.lng, 14);
  let finish_tile_y = lat2tile(finish.lat, 14);

  for (let x = Math.min(start_tile_x, finish_tile_x); x <= Math.max(start_tile_x, finish_tile_x); x++) {
    for (let y = Math.min(start_tile_y, finish_tile_y); y <= Math.max(start_tile_y, finish_tile_y); y++) {
      let intersections = pm_intersections.getZxy(14, x, y)
      let roads = pm_roads.getZxy(14, x, y)
      ingest_tile(x, y, 14, new Uint8Array((await roads).data), new Uint8Array((await intersections).data), costing_model.cost_intersection, costing_model.cost_way);
    }
  }

  const encoded = search(start.lng, start.lat, finish.lng, finish.lat);
  if (encoded) {
    postMessage(toGeoJSON(encoded));
  } else {
    postMessage(null);
  }
}, 1000);

self.onmessage = async (event) => {
  await process_event(event)
};

postMessage("ready")

