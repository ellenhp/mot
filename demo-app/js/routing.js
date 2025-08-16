import { main_js } from '../pkg/index.js';
import { toGeoJSON } from '@mapbox/polyline';
import debounce from 'debounce';

let wasm_promise = import("../pkg/index.js").catch(console.error);

main_js();

const process_event = debounce(async (event) => {
  console.log(event)
  const costing_model = eval(event.data);
  let tile = await fetch("http://localhost:3000/intersections,roads/14/2623/5718");
  let tile_bytes = await (await tile.blob()).bytes();
  let wasm = await wasm_promise;
  console.log("ingesting tile");
  wasm.clear();
  wasm.ingest_tile(2623, 5718, 14, tile_bytes, costing_model.cost_intersection, costing_model.cost_way);
  const encoded = wasm.search(BigInt(671949014), 0, BigInt(980366562), 0);
  postMessage(toGeoJSON(encoded));
  console.log(encoded);
  if (encoded) {
  }
}, 200);

self.onmessage = async (event) => {
  process_event(event)
};

postMessage(null)

