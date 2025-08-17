# MapLibre Offline Toolkit

### What this project is

At some point in 2025 I realized I have personally written the majority of a maps stack, mostly because I rewrite stuff in Rust compulsively as a hobby. This project is the culimation of that effort. I've decided that hosting infrastructure is hard, so I'm focusing on doing everything offline-first. The geocoder, `mvts`, is mostly pulled from my other project, [airmail](https://github.com/ellenhp/airmail), except it's modified to index data directly from a vector tileset. So if you have the tiles downloaded, the geocoder functions. The routing engine, `mvtr` works in the same way, except preparing the tileset is much more involved, requiring a bunch of PostGIS processing.

The geocoder works pretty well, but the routing engine needs a lot of work.

### Testimonials

"Another piece of abandonware, Ellen? Really?"

-- anonymous

"So it's like CoMaps but worse?"

-- anonymous

