DROP TABLE IF EXISTS poi;

CREATE TABLE poi (
  way_id BIGINT,
  node_id BIGINT,
  tags JSONB,
  geom GEOMETRY
);

SELECT UpdateGeometrySRID('poi', 'geom', 4326);

INSERT INTO poi (way_id, tags, geom) SELECT way_id, tags, geom FROM way_poi;
INSERT INTO poi (node_id, tags, geom) SELECT node_id, tags, geom FROM node_poi;

CREATE INDEX poi_geom_idx ON poi USING GIST(geom);
CREATE INDEX poi_wid_idx ON poi (way_id);
CREATE INDEX poi_nid_idx ON poi (node_id);

CREATE TABLE IF NOT EXISTS wof_admins (
  id BIGINT,
  geom GEOMETRY(MULTIPOLYGON, 4326),
  admin_level TEXT,
  names TEXT
);

CREATE INDEX IF NOT EXISTS wof_admins_geom ON wof_admins USING GIST(geom);

DROP TABLE IF EXISTS tiles;
CREATE TABLE tiles (
  idx INTEGER,
  x INTEGER,
  y INTEGER,
  z INTEGER,
  geom GEOMETRY(POLYGON, 4326)
);

WITH tile_coords AS
(
  select generate_series as idx, floor(generate_series/4096)::integer as x, (generate_series%4096)::integer as y from generate_series(0, 16777215)
)
INSERT INTO tiles (
  idx, x, y, z, geom
)
SELECT
  tile_coords.idx as idx,
  tile_coords.x as x,
  tile_coords.y as y,
  12 as z,
  ST_Transform(ST_TileEnvelope(12, tile_coords.x, tile_coords.y), 4326) as geom
FROM tile_coords;

CREATE INDEX IF NOT EXISTS tiles_geom ON tiles USING GIST(geom);

