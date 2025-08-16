
let wasm_promise = import("../pkg/index.js").catch(console.error);

window.addEventListener('load', async function () {
  let tile = await fetch("http://localhost:3000/intersections,roads/14/2623/5718");
  let tile_bytes = await (await tile.blob()).bytes();
  let wasm = await wasm_promise;
  console.log("ingesting tile");
  // wasm.ingest_tile(tile_bytes, function () {
  //   console.log(a);
  //   console.log(b);
  // }, function (a, b) {
  //   console.log(a);
  // })
  // console.log("cost2");
})