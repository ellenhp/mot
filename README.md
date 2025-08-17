# MapLibre Offline Toolkit

### Background

Reinventing the wheel has been a hobby of mine for some time, and at some point in 2025 I realized I had written nearly half of a maps stack. I wrote a [geocoder](https://github.com/ellenhp/airmail) and a [transit routing engine](https://github.com/ellenhp/solari) already. Other notable components include routing and vector tile generation and rendering. Basemap rendering is not an area I plan to tackle because the FOSS maps ecosystem has more or less standardized on MapLibre and Mapnik as the leading tech, and rebuilding either of them sounds very difficult for minimal gain. I also don't care to rewrite Planetiler in Rust for similar reasons. Routing on the other hand isn't nearly as hard a problem at is core; it's just a graph search right? (wrong, mostly) While there are a few existing options, most of them are written in C or C++ which makes them difficult to use from the ecosystems I tend to work in for hobby projects. For online use, Graphhopper, OSRM or Valhalla are the top contenders. For offline mobile applications, the best existing option is valhalla-mobile or lifting something from CoMaps or OsmAnd.

Author's note: the client-side geocoding landscape is even worse. To my knowledge there isn't any standalone project out there for offline geocodng, so you get to either build your own, adapt something else, or lift code from an existing offline-first maps app.

### This project

`mot` is my attempt to fill in the gaps in the client-side maps space. While there are a few notable FOSS mobile apps out there that do a pretty good job of handling offline mobile use-cases, they aren't modular enough to pull into other projects easily, nor do they make much use of industry-standard technology. This is why the rendering and basemap UX in OsmAnd feel "weird"--it's all bespoke as opposed to everything else out there which is just using Google or MapLibre or similar. I believe pretty firmly that copying the design language and look-and-feel from major players in the space is necessary to appeal to the masses.

There are a few components currently. `mvts` is a geocoder based on [airmail](https://github.com/ellenhp/airmail) that creates a search index by ingesting POIs from slightly modified vector tiles. There are no separate downloads required to make it work. `mvtr` is a routing engine built on the same principle, but there are some additional processing steps in PostGIS to generate the tiles that it requires. The geocoder works reasonably well already, but the routing engine needs a lot of work. The core algorithms have a few interesting bugs currently and the costing models have yet to be developed. The development of good costing models is something companies can and have put engineer-centuries into. It's not something that's every really finished.

### In-scope

I intend to pull in parts of [`solari`](https://github.com/ellenhp/solari) to enable offline transit routing. [mobroute](https://mr.lrdu.org/mobroute/) already exists so this isn't totally groundbreaking but transit is a nice thing to include.

### Not in-scope

This project will never become a maps app. I may end up building one based on it in the future, but my goal here is something modular and standalone.