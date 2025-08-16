import { main_js } from '../pkg/index.js';
import { toGeoJSON } from '@mapbox/polyline';
import debounce from 'debounce';
import { PMTiles } from 'pmtiles';

let wasm_promise = import("../pkg/index.js").catch(console.error);

main_js();

const process_event = debounce(async (event) => {
  const costing_model = eval(event.data.costing_model);
  let pm_intersections = new PMTiles(event.data.url + "seattle_intersections.pmtiles")
  let pm_roads = new PMTiles(event.data.url + "seattle_roads.pmtiles")

  let intersections = pm_intersections.getZxy(14, 2623, 5718)
  let roads = pm_roads.getZxy(14, 2623, 5718)
  let wasm = await wasm_promise;
  wasm.clear();

  wasm.ingest_tile(2623, 5718, 14, new Uint8Array((await roads).data), new Uint8Array((await intersections).data), costing_model.cost_intersection, costing_model.cost_way);
  const encoded = wasm.search(BigInt(671949014), 0, BigInt(980366562), 0);
  if (encoded) {
    postMessage(toGeoJSON(encoded));
  } else {
    postMessage(null);
  }
}, 200);

self.onmessage = async (event) => {
  await process_event(event)
};

postMessage("ready")

