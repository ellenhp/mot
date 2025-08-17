import { toGeoJSON } from '@mapbox/polyline';
import debounce from 'debounce';
import { PMTiles } from 'pmtiles';

import { main_js, clear, ingest_tile, search } from '../pkg/index.js';

main_js();

const process_event = debounce(async (event) => {
  const costing_model = eval(event.data.costing_model);
  const start = event.data.start;
  const finish = event.data.finish;
  let pm_intersections = new PMTiles(event.data.url + "seattle_intersections.pmtiles")
  let pm_roads = new PMTiles(event.data.url + "seattle_roads.pmtiles")

  let intersections = pm_intersections.getZxy(14, 2623, 5718)
  let roads = pm_roads.getZxy(14, 2623, 5718)
  clear();

  ingest_tile(2623, 5718, 14, new Uint8Array((await roads).data), new Uint8Array((await intersections).data), costing_model.cost_intersection, costing_model.cost_way);
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

